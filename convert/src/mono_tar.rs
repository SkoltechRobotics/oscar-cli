use crate::cli::{ConvertOpt, Format};
use crate::utils::{save_img, get_timestamp};
use std::{io, fs, error, thread};
use std::io::{Read, Write};
use std::path::{PathBuf, Path};
use indicatif::{ProgressBar, ProgressStyle};

use oscar_utils::{WIDTH, HEIGHT};

const TEMPLATE: &str = "\
    {wide_bar} {percent:>3}% {bytes}/{total_bytes} \
    Elapsed: {elapsed_precise} ETA: {eta_precise}\
";

fn worker(pos: usize, data: Box<[u8]>, opt: &ConvertOpt) {
    let res = oscar_utils::load_frames::decode_flif(&data)
        .and_then(|img_data| {
            let file_name = format!("{:#06}", pos);
            save_img(
                &file_name, img_data, &opt.format, &opt.output,
                WIDTH as u32, HEIGHT as u32,
            )
        });
    if let Err(err) = res {
        eprintln!("Error: {} {}\n", pos, err);
    }
}

/// Save index data to TSV file
fn save_index(index: Vec<(usize, PathBuf)>, dir: &Path) -> io::Result<()>{
    let index_path = dir.join("index.tsv");
    let mut index_file = io::BufWriter::new(fs::File::create(index_path)?);
    index_file.write_all(
        b"N\tUNIX time, ms\tOS time, ms\tPrevious frame dt, ms\n"
    )?;

    let mut t_prev = get_timestamp(&index[0].1)?.os;
    for (n, path) in index {
        let t = get_timestamp(&path)?;
        write!(index_file, "{:#06}\t{}\t{}\t{}\n",
            n, t.unix, t.os, t.os - t_prev)?;
        t_prev = t.os;
    }
    Ok(())
}

pub fn convert(opt: ConvertOpt) -> Result<(), Box<dyn error::Error>> {
    if !opt.format.demosaic {
        if opt.format.scale != 1 {
            Err("can't downscale image without demosaicing")?
        }
        if opt.format.format == Format::Jpeg {
            Err("don't use JPEG without demosaicing")?
        }
        if opt.format.histeq {
            Err("can't apply histogram equalization without demosaicing")?
        }
    }
    println!("Processing: {}", opt.input);

    let (reader, tar_size) = if opt.input.starts_with("http://") {
        let r = reqwest::get(&opt.input)?;
        let n = r.content_length().expect("expected valid content length");
        (Box::new(r) as Box<dyn Read>, n)
    } else {
        let f = fs::File::open(&opt.input)?;
        let n = f.metadata()?.len();
        (Box::new(f) as Box<dyn Read>, n)
    };
    let mut input_tar = tar::Archive::new(reader);

    fs::create_dir_all(&opt.output)?;

    let mut index = Vec::new();

    let num = num_cpus::get();
    let (frames_in, frames_out) = crossbeam_channel::bounded(2*num);

    let handles: Vec<_> = (0..num)
        .map(|_| {
            let rx = frames_out.clone();
            let opt = opt.clone();
            thread::spawn(move|| {
                for (pos, data) in rx {
                    worker(pos, data, &opt);
                }
            })
        })
        .collect();

    let bar = ProgressBar::new(tar_size);
    bar.set_style(ProgressStyle::default_bar().template(TEMPLATE));

    for (pos, file) in input_tar.entries()?.enumerate() {
        let mut file = file?;
        let path = file.header().path()?;
        let size = file.header().size()?;
        bar.set_position(file.raw_file_position() + size);

        index.push((pos, path.into_owned()));

        if pos < opt.skip as usize { continue; }

        let n = index.len();
        if n > 2 { assert!(index[n-2] < index[n-1], "files are not ordered"); }

        let mut buf = Vec::with_capacity(size as usize);
        file.read_to_end(&mut buf)?;

        frames_in.send((pos, buf.into_boxed_slice()))?;
    }
    drop(frames_in);
    for handle in handles {
        handle.join().expect("failed to join thread");
    }
    bar.finish();
    save_index(index, &opt.output)?;

    Ok(())
}
