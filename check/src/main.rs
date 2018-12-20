use std::path::{Path, PathBuf};
use std::io;

use structopt::StructOpt;
use indicatif::{ProgressBar, ProgressStyle, ParallelProgressIterator};
use rayon::iter::{ParallelIterator, IntoParallelRefIterator};

use oscar_utils::PBAR_TEMPLATE;
use oscar_utils::conversions::raw_flip;
use oscar_utils::load_frames::{load_raw_pnm, load_flif};

#[derive(StructOpt)]
#[structopt(
    name = "check",
    about = "Checks that raw Bayer PNM and RGBA FLIF contain the same data")]
pub(crate) struct Cli {
    #[structopt(parse(from_os_str))]
    pnm_dir: PathBuf,
    #[structopt(parse(from_os_str))]
    flif_dir: PathBuf,
}

fn get_filenames(dir: &Path, ext: &str) -> io::Result<Vec<String>> {
    let ext = std::ffi::OsStr::new(ext);
    let mut fnames = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let ft = entry.file_type()?;
        if !ft.is_file() || path.extension() != Some(ext) { continue; }
        let fname = path.file_stem()
            .expect("failed to get file stem")
            .to_str()
            .expect("non-UTF-8 file name")
            .to_string();
        fnames.push(fname)
    }
    fnames.sort();
    Ok(fnames)
}

fn compare(fname: &str, flif_dir: &Path, pnm_dir: &Path) -> io::Result<bool> {
    let flif_path = flif_dir.join(&fname).with_extension("flif");
    let pnm_path = pnm_dir.join(&fname).with_extension("pnm");

    let pnm_frame = load_raw_pnm(&pnm_path)?;
    let mut flif_frame = load_flif(&flif_path)?;
    raw_flip(&mut flif_frame);

    Ok(&pnm_frame[..] == &flif_frame[..])
}

fn main() -> io::Result<()> {
    let args = Cli::from_args();
    let flifs = get_filenames(&args.flif_dir, "flif")?;
    let pnms = get_filenames(&args.pnm_dir, "pnm")?;
    if flifs != pnms {
        panic!("extra or missing files");
    }
    let fnames = flifs;

    let bar = ProgressBar::new(fnames.len() as u64);
    bar.set_style(ProgressStyle::default_bar().template(PBAR_TEMPLATE));

    let res: Vec<String> = fnames.par_iter()
        .progress_with(bar)
        .map(|fname| compare(fname, &args.flif_dir, &args.pnm_dir))
        .collect::<io::Result<Vec<bool>>>()?
        .iter()
        .zip(fnames.into_iter())
        .filter(|(&is_equal, _)| !is_equal)
        .map(|(_, fname)| fname)
        .collect();

    if res.len() != 0 {
        println!("{:?} frame(s) are not equal to each other.", res.len());
        for fname in res { println!("{}", fname); }
    }

    Ok(())
}