use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::{io, fs, thread, error};

use {num_cpus, pbr};

use opt::ConvertOpt;
use utils::{read_flif, save_img, get_timestamp, Timestamp};

type MonoIndex = Vec<(usize, PathBuf, Timestamp)>;

fn construct_index(dir_path: &Path) -> io::Result<MonoIndex> {
    print!("Building list of images... ");
    io::stdout().flush()?;
    let mut index = fs::read_dir(dir_path)?
        .map(|entry| {
            let path = entry?.path();
            let t = get_timestamp(&path)?;
            Ok((path, t))
        })
        .collect::<io::Result<Vec<(PathBuf, Timestamp)>>>()?;
    index.sort_unstable_by(|a, b| a.1.os.cmp(&b.1.os));
    let index: MonoIndex = index.into_iter()
        .enumerate()
        .rev()
        .map(|(i, e)| (i, e.0, e.1))
        .collect();
    println!("Done. Images found: {}", index.len());
    Ok(index)
}

enum WorkerMessage {
    Ok(PathBuf),
    Err(PathBuf, io::Error),
    Done,
}

fn worker(
    index: &Arc<Mutex<MonoIndex>>, chan: &mpsc::Sender<WorkerMessage>,
    opt: &ConvertOpt,
) {
    let index = index.clone();
    let chan = chan.clone();
    let opt = opt.clone();
    thread::spawn(move|| {
        loop {
            let (n, path, _) = match index.lock().expect("mutex failure").pop() {
                Some(val) => val,
                None => break,
            };
            let res = read_flif(&path)
                .and_then(|img_data| {
                    let file_name = format!("{}", n);
                    save_img(
                        &file_name, img_data, &opt.format, &opt.output,
                        2448, 2048,
                    )
                });

            let msg = match res {
                Ok(()) => WorkerMessage::Ok(path),
                Err(err) => WorkerMessage::Err(path, err),
            };
            chan.send(msg).expect("channel failure");
        }
        chan.send(WorkerMessage::Done).expect("channel failure");
    });
}

/// Save index data to TSV file
fn save_index(index: &MonoIndex, dir: &Path) -> io::Result<()>{
    let mut index_path = dir.to_path_buf();
    index_path.push("index.tsv");
    let mut index_file = io::BufWriter::new(fs::File::create(index_path)?);
    index_file.write_all(
        b"N\tUNIX time, ms\tOS time, ms\tPrevious frame dt, ms\n"
    )?;

    let i = index.len() - 1;
    let mut t_prev = get_timestamp(&index[i].1)?.os;
    for (n, _, t) in index.iter().rev() {
        write!(index_file, "{}\t{}\t{}\t{}\n", n, t.unix, t.os, t.os - t_prev)?;
        t_prev = t.os;
    }
    Ok(())
}

pub fn convert(opt: ConvertOpt) -> Result<(), Box<error::Error>> {
    if !opt.format.demosaic && opt.format.scale != 1 {
        Err("can't downscale image without demosaicing")?
    }
    println!("Processing: {}", opt.input.display());
    let mut index = construct_index(&opt.input)?;
    fs::create_dir_all(&opt.output)?;
    save_index(&index, &opt.output)?;

    let n = index.len();
    index.truncate(n - opt.skip as usize);

    let count = index.len() as u64;
    let index = Arc::new(Mutex::new(index));
    let (sender, receiver) = mpsc::channel();
    let mut workers = opt.workers.unwrap_or_else(|| num_cpus::get() as u8);

    for _ in 0..workers {
        worker(&index, &sender, &opt);
    }
    let mut pb = pbr::ProgressBar::new(count);
    let mut counter = 0;
    loop {
        match receiver.recv() {
            Ok(WorkerMessage::Ok(_path)) => {
                counter = pb.inc();
            },
            Ok(WorkerMessage::Err(path, err)) => {
                counter = pb.inc();
                eprintln!("error [{}] {}", path.display() ,err);
            },
            Ok(WorkerMessage::Done) => {
                workers -= 1;
                if workers == 0 { break; }
            },
            Err(_) => break,
        }
    }
    assert_eq!(counter, count, "not all messages processed");
    pb.finish_print("Completed");

    Ok(())
}