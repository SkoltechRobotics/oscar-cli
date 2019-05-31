use std::io::Write;
use std::path::{Path, PathBuf};
use std::{io, fs, error};

use indicatif::{ProgressBar, ProgressStyle, ParallelProgressIterator};
use rayon::iter::{ParallelIterator, IntoParallelRefIterator};

use super::cli::{ConvertOpt, Format};
use super::utils::{save_img, get_timestamp, Timestamp};
use oscar_utils::load_frames::load_flif;
use oscar_utils::{WIDTH, HEIGHT, PBAR_TEMPLATE};

type MonoIndex = Vec<(usize, PathBuf, Timestamp)>;

fn construct_index(dir_path: &str) -> io::Result<MonoIndex> {
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
        write!(index_file, "{:#06}\t{}\t{}\t{}\n",
            n, t.unix, t.os, t.os - t_prev)?;
        t_prev = t.os;
    }
    Ok(())
}

pub fn convert(opt: ConvertOpt) -> Result<(), Box<error::Error>> {
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
    let mut index = construct_index(&opt.input)?;
    fs::create_dir_all(&opt.output)?;
    save_index(&index, &opt.output)?;

    let n = index.len();
    index.truncate(n - opt.skip as usize);

    let bar = ProgressBar::new(index.len() as u64);
    bar.set_style(ProgressStyle::default_bar().template(PBAR_TEMPLATE));
    index.par_iter()
        .progress_with(bar)
        .for_each(|(n, path, _)| {
            let res = load_flif(&path)
                .and_then(|img_data| {
                    let file_name = format!("{:#06}", n);
                    save_img(
                        &file_name, img_data, &opt.format, &opt.output,
                        WIDTH as u32, HEIGHT as u32,
                    )
                });
            if let Err(err) = res {
                eprintln!("Error: {:?} {}\n", path, err);
            }
        });

    Ok(())
}