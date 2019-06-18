use std::io::Write;
use std::path::{Path, PathBuf};
use std::{io, fs, cmp, error};

use indicatif::{ProgressBar, ProgressStyle, ParallelProgressIterator};
use rayon::iter::{ParallelIterator, IntoParallelRefIterator};

use super::cli::{ConvertStereoOpt, Format};
use super::utils::{save_stereo_img, get_timestamps, Timestamp};
use oscar_utils::load_frames::load_flif;
use oscar_utils::{WIDTH, HEIGHT, PBAR_TEMPLATE};

const FPS: u64 = 30;

type Pair = (Option<Timestamp>, Option<Timestamp>);
type StereoIndex = Vec<(usize, Pair)>;


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

    assert!((ts_l[0].os as isize - ts_r[0].os as isize).abs() < 10);

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
fn read_flif2(ts: Option<Timestamp>, dir: &Path) -> io::Result<Box<[u8]>> {
    match ts {
        Some(ts) => load_flif(&to_path(dir, ts)),
        None => Ok(vec![0; WIDTH*HEIGHT].into_boxed_slice()),
    }
}

/// Save index data to TSV file
fn save_index(index: &StereoIndex, dir: &Path) -> io::Result<()>{
    let mut index_path = dir.to_path_buf();
    index_path.push("index.tsv");
    let mut index_file = io::BufWriter::new(fs::File::create(index_path)?);
    index_file.write_all(
        b"Pair number\tLeft frame UNIX time, us\tRight frame UNIX time, us\t\
        Left frame OS time, us\tRight frame OS time, us\t\
        UNIX time delta, us\tOS time delta, us\n"
    )?;

    for (n, pair) in index.iter().rev() {
        let line = match pair {
            (Some(l), Some(r)) => format!("{:#06}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                n, l.unix, r.unix, l.os, r.os,
                l.unix as i64 - r.unix as i64, l.os as i64 - r.os as i64 ),
            (Some(l), None) => format!("{:#06}\t{}\t\t{}\t\t\t\n", n, l.unix, l.os),
            (None, Some(r)) => format!("{:#06}\t\t{}\t\t{}\t\t\n", n, r.unix, r.os),
            (None, None) => format!("{:#06}\t\t\t\t\t\t\n", n),
        };
        index_file.write_all(line.as_bytes())?;
    }
    Ok(())
}

pub fn convert(opt: ConvertStereoOpt) -> Result<(), Box<dyn error::Error>> {
    if !opt.format.demosaic && opt.format.scale != 1 {
        Err("can't downscale image without demosaicing")?
    }
    if !opt.format.demosaic && opt.format.format == Format::Jpeg {
        Err("don't use JPEG without demosaicing")?
    }
    println!("Processing: {}", opt.input.display());
    let mut index = construct_index(&opt)?;
    fs::create_dir_all(&opt.output)?;
    save_index(&index, &opt.output)?;

    let n = index.len();
    index.truncate(n - opt.skip as usize);

    let bar = ProgressBar::new(index.len() as u64);
    bar.set_style(ProgressStyle::default_bar().template(PBAR_TEMPLATE));
    index.par_iter()
        .progress_with(bar)
        .for_each(|(n, pair,)| {
            let skip = match pair {
                (Some(_), Some(_)) => false,
                (None,  None) => opt.ignore_empty,
                _ => opt.ignore_partial,
            };
            if skip { return; }

            // TODO: optimize
            let mut left = opt.input.clone();
            left.push("left");
            let mut right = opt.input.clone();
            right.push("right");

            let res = read_flif2(pair.0, &left)
                .and_then(|left| Ok((left, read_flif2(pair.1, &right)?)))
                .and_then(|(left_img, right_img)| {
                    let file_name = format!("{:#06}", n);
                    save_stereo_img(
                        &file_name, left_img, right_img,
                        &opt.format, &opt.output,
                        WIDTH as u32, HEIGHT as u32,
                    )
                });

            if let Err(err) = res {
                println!("Error: {:?} {}\n", pair, err);
            }
        });

    Ok(())
}
