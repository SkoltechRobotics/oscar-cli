use std::io::Write;
use std::sync::{Arc, Mutex, mpsc};
use std::path::{Path, PathBuf};
use std::{io, fs, cmp, thread, error};

use {num_cpus, pbr};

use opt::ConvertStereoOpt;
use utils::{read_flif, save_img};

const FPS: u64 = 30;

#[derive(Copy, Clone, Debug)]
struct Timestamp {
    unix: u64,
    os: u64,
}

type Pair = (Option<Timestamp>, Option<Timestamp>);
type StereoIndex = Vec<(usize, Pair)>;

enum WorkerMessage {
    Ok(Pair),
    Err(Pair, io::Error),
    Done,
}

// TODO replace panics with errors
fn get_timestamps(dir_path: &Path) -> io::Result<Vec<Timestamp>> {
    let iter = fs::read_dir(dir_path)?;
    let mut buf = Vec::with_capacity(iter.size_hint().0);
    for entry in iter {
        let path = entry?.path();
        if !path.is_file() {
            panic!("got dir")
        }
        match path.extension() {
            Some(ext) if ext == "flif" => (),
            _ => panic!("temp"),
        }
        let file_name = match path.file_stem() {
            Some(val) => val.to_str().expect("non-UTF8 path"),
            None => panic!("incorrect path: {}", dir_path.display()),
        };
        let mut iter = file_name.split('_').map(|v| v.parse::<u64>());
        let ts = match (iter.next(), iter.next(), iter.next()) {
            (Some(Ok(unix)), Some(Ok(os)), None) => Timestamp { unix, os },
            _ => panic!("incorrect filename pattern"),
        };
        buf.push(ts);
    }
    buf.sort_unstable_by(|a, b| a.os.cmp(&b.os));
    Ok(buf)
}

fn to_path(dir: &Path, ts: Timestamp) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.push(format!("{}_{}.flif", ts.unix, ts.os));
    path
}

fn grow_index_half(index: &mut Vec<Option<Timestamp>>, ts: &[Timestamp]) {
    index.push(Some(ts[0]));

    let mut prev_ts = ts[0].os;
    for ts in ts.iter().skip(1) {
        let t = ts.os;
        let dt = (((t - prev_ts)*FPS) as f32/1000.0).round() as usize;
        prev_ts = t;

        assert!(dt >= 1);
        for _ in 0..dt - 1 { index.push(None) }
        index.push(Some(*ts))
    }
}

fn construct_index(opt: &ConvertStereoOpt) -> io::Result<StereoIndex> {
    print!("Building list of images... ");
    io::stdout().flush()?;

    let mut left = opt.input.clone();
    left.push("left");
    let mut right = opt.input.clone();
    right.push("right");

    let ts_l = get_timestamps(&left)?;
    let ts_r = get_timestamps(&right)?;

    assert!((ts_l[0].os as isize - ts_r[0].os as isize).abs() < 5);

    let nl = ts_l.len();
    let nr = ts_r.len();
    let n = cmp::max(ts_l[nl-1].os - ts_l[0].os, ts_r[nr-1].os - ts_r[0].os);
    let mut index_l = Vec::with_capacity((n/FPS) as usize);
    let mut index_r = Vec::with_capacity((n/FPS) as usize);

    grow_index_half(&mut index_l, &ts_l);
    grow_index_half(&mut index_r, &ts_r);

    let nl = index_l.len();
    let nr = index_r.len();
    if nl > nr {
        for _ in 0..nl-nr { index_r.push(None); }
    } else if nr > nl {
        for _ in 0..nr-nl { index_l.push(None); }
    }

    let mut counter_full = 0u32;
    let mut counter_part = 0u32;
    let mut counter_empty = 0u32;

    let mut res: StereoIndex = index_l.drain(0..)
        .zip(index_r.drain(..))
        .filter(|(l, r)| {
            match (l, r) {
                (Some(l), Some(r)) => {
                    let delta = ((l.os as i64) - (r.os as i64)).abs();
                    assert!(delta < 10);
                    counter_full += 1;
                    true
                },
                (None, None) => {
                    counter_empty += 1;
                    !opt.ignore_empty
                },
                _ => {
                    counter_part += 1;
                    !opt.ignore_partial
                },
            }
        })
        .enumerate()
        .collect();

    res.reverse();

    println!("Done. Pairs: full {}, partial {}, empty {}",
        counter_full, counter_part, counter_empty);

    Ok(res)
}

/// returns empty image if `ts` is None
fn read_flif2(ts: Option<Timestamp>, dir: &Path) -> io::Result<Vec<u8>> {
    match ts {
        Some(ts) => read_flif(&to_path(dir, ts)),
        None => Ok(vec![0; 2448*2048]),
    }
}

fn concat_images(left: Vec<u8>, right: Vec<u8>) -> Vec<u8> {
    assert_eq!(left.len(), 2448*2048);
    assert_eq!(right.len(), 2448*2048);
    let mut out = vec![0; 2*2448*2048];
    let width = 2448;
    for (i, (l, r)) in left.iter().zip(right.iter()).enumerate() {
        let x = i % 2448;
        let y = i / 2448;
        out[2*width*y + x] = *l;
        out[2*width*y + x + width] = *r;
    }
    out
}

fn worker(
    files: &Arc<Mutex<StereoIndex>>, chan: &mpsc::Sender<WorkerMessage>,
    opt: ConvertStereoOpt,
) {
    let files = files.clone();
    let chan = chan.clone();
    thread::spawn(move|| {
        let mut left = opt.input.clone();
        left.push("left");
        let mut right = opt.input.clone();
        right.push("right");
        loop {
            let (n, pair) = match files.lock().expect("mutex failure").pop() {
                Some(path) => path,
                None => break,
            };
            let skip = match pair {
                (Some(_), Some(_)) => false,
                (None,  None) => opt.ignore_empty,
                _ => opt.ignore_partial,
            };
            if skip { break }

            let res = read_flif2(pair.0, &left)
                .and_then(|left| Ok((left, read_flif2(pair.1, &right)?)))
                .and_then(|(left_img, right_img)| {
                    let file_name = format!("{}", n);
                    let img_data = concat_images(left_img, right_img);
                    save_img(
                        &file_name, img_data, &opt.format, &opt.output,
                        2*2448, 2048,
                    )
                });

            let msg = match res {
                Ok(()) => WorkerMessage::Ok(pair),
                Err(err) => WorkerMessage::Err(pair, err),
            };
            chan.send(msg).expect("channel failure");
        }
        chan.send(WorkerMessage::Done).expect("channel failure");
    });
}

/// Save index data to TSV file
fn save_index(index: &StereoIndex, dir: &Path) -> io::Result<()>{
    let mut index_path = dir.to_path_buf();
    index_path.push("index.tsv");
    let mut index_file = io::BufWriter::new(fs::File::create(index_path)?);
    index_file.write_all(
        b"Pair number\tLeft frame UNIX time, ms\tRight frame UNIX time, ms\t\
        Left frame OS time, ms\tRight frame OS time, ms\t\
        UNIX time delta, ms\tOS time delta, ms\n"
    )?;

    for (n, pair) in index.iter().rev() {
        let line = match pair {
            (Some(l), Some(r)) => format!("{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                n, l.unix, r.unix, l.os, r.os,
                l.unix as i64 - r.unix as i64, l.os as i64 - r.os as i64 ),
            (Some(l), None) => format!("{}\t{}\t\t{}\t\t\t\n", n, l.unix, l.os),
            (None, Some(r)) => format!("{}\t\t{}\t\t{}\t\t\n", n, r.unix, r.os),
            (None, None) => format!("{}\t\t\t\t\t\t\n", n),
        };
        index_file.write_all(line.as_bytes())?;
    }
    Ok(())
}

pub fn convert(opt: ConvertStereoOpt) -> Result<(), Box<error::Error>> {
    if !opt.format.demosaic && opt.format.scale != 1 {
        Err("can't downscale image without demosaicing")?
    }
    println!("Processing: {}", opt.input.display());
    let mut index = construct_index(&opt)?;
    fs::create_dir_all(&opt.output)?;
    save_index(&index, &opt.output)?;

    let n = index.len();
    index.truncate(n - opt.skip as usize);

    let count = index.len() as u64;
    let index = Arc::new(Mutex::new(index));
    let (sender, receiver) = mpsc::channel();
    let mut workers = opt.workers.unwrap_or_else(|| num_cpus::get() as u8);

    for _ in 0..workers {
        worker(&index, &sender, opt.clone());
    }
    let mut pb = pbr::ProgressBar::new(count);
    let mut counter = 0;
    loop {
        match receiver.recv() {
            Ok(WorkerMessage::Ok(_path)) => {
                counter = pb.inc();
            },
            Ok(WorkerMessage::Err(pair, err)) => {
                counter = pb.inc();
                eprintln!("error [{:?}] {}", pair ,err);
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
