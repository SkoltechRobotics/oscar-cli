use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::{io, fs, thread, error};

use {num_cpus, pbr};

use opt::ConvertOpt;
use utils::{read_flif, save_img};

fn construct_index(dir_path: &Path) -> io::Result<Vec<PathBuf>> {
    print!("Building list of images... ");
    io::stdout().flush()?;
    let mut files = fs::read_dir(dir_path)?
        .map(|entry| {
            let path = entry?.path();
            if !path.is_file() {
                panic!("got dir")
            }
            match path.extension() {
                Some(ext) if ext == "flif" => (),
                _ => panic!("temp"),
            }
            // TODO: add filename pattern check
            Ok(path)
        })
        .collect::<io::Result<Vec<PathBuf>>>()?;
    // it assumes that image follows naming pattern and was taken between
    // 2001-09-09 and 2286-11-20 (1M and 10M seconds of UNIX epoch respectively)
    files.sort_unstable_by(|a, b| b.cmp(a));
    println!("Done. Images found: {}", files.len());
    Ok(files)
}

enum WorkerMessage {
    Ok(PathBuf),
    Err(PathBuf, io::Error),
    Done,
}

fn worker(
    files: &Arc<Mutex<Vec<PathBuf>>>, chan: &mpsc::Sender<WorkerMessage>,
    opt: ConvertOpt,
) {
    let files = files.clone();
    let chan = chan.clone();
    thread::spawn(move|| {
        loop {
            let path = match files.lock().expect("mutex failure").pop() {
                Some(path) => path,
                None => break,
            };
            let res = read_flif(&path)
                .and_then(|img_data| {
                    let file_name = path.file_name()
                        .expect("file names already checked")
                        .to_str()
                        .expect("non-UTF8 filename");
                    save_img(
                        file_name, img_data, &opt.format, &opt.output,
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

pub fn convert(opt: ConvertOpt) -> Result<(), Box<error::Error>> {
    if !opt.format.demosaic && opt.format.scale != 1 {
        Err("can't downscale image without demosaicing")?
    }
    println!("Processing: {}", opt.input.display());
    let mut files = construct_index(&opt.input)?;
    fs::create_dir_all(&opt.output)?;

    let n = files.len();
    files.truncate(n - opt.skip as usize);

    let count = files.len() as u64;
    let files = Arc::new(Mutex::new(files));
    let (sender, receiver) = mpsc::channel();
    let mut workers = opt.workers.unwrap_or_else(|| num_cpus::get() as u8);

    for _ in 0..workers {
        worker(&files, &sender, opt.clone());
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