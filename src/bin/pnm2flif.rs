use std::path::{Path, PathBuf};
use std::{io, fs};

use structopt::StructOpt;
use indicatif::{ProgressBar, ProgressStyle, ParallelProgressIterator};
use rayon::iter::{ParallelIterator, IntoParallelRefIterator};

use oscar_utils::{convert_pnm2flif, PBAR_TEMPLATE};

#[derive(StructOpt)]
#[structopt(
    name = "pnm2flif",
    about = "Convert raw Bayer PNM frames to RGBA FLIF")]
pub(crate) struct Cli {
    #[structopt(parse(from_os_str))]
    src_dir: PathBuf,
    #[structopt(parse(from_os_str))]
    dst_dir: PathBuf,
}

fn main() -> io::Result<()> {
    let args = Cli::from_args();

    fs::create_dir_all(&args.dst_dir)?;
    let pnm_ext = std::ffi::OsStr::new("pnm");

    let mut tasks: Vec<(PathBuf, PathBuf)> = Default::default();
    for entry in std::fs::read_dir(args.src_dir)? {
        let entry = entry?;
        let src_path = entry.path();
        let ft = entry.file_type()?;
        if !ft.is_file() || src_path.extension() != Some(pnm_ext) { continue; }
        let dst_fname = Path::new(src_path.file_name().unwrap().into())
            .with_extension("flif");
        let dst_path = args.dst_dir.join(dst_fname);
        tasks.push((src_path, dst_path));
    }

    let bar = ProgressBar::new(tasks.len() as u64);
    bar.set_style(ProgressStyle::default_bar().template(PBAR_TEMPLATE));
    tasks.par_iter()
        .progress_with(bar)
        .for_each(|(src_path, dst_path)| {
            convert_pnm2flif(src_path, dst_path).expect("conversion failed");
        });

    Ok(())
}
