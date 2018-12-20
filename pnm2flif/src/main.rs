use std::path::{Path, PathBuf};
use std::{io, fs};
use std::io::Write;
use std::process::{Command, Stdio};

use structopt::StructOpt;
use indicatif::{ProgressBar, ProgressStyle, ParallelProgressIterator};
use rayon::iter::{ParallelIterator, IntoParallelRefIterator};

use oscar_utils::{PBAR_TEMPLATE, WIDTH, HEIGHT};
use oscar_utils::load_frames::load_raw_pnm;
use oscar_utils::conversions::raw2rgba_flip;

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

const PAM_HEADER: &[u8] = b"\
    P7\n\
    WIDTH 1224\n\
    HEIGHT 1024\n\
    DEPTH 4\n\
    MAXVAL 255\n\
    TUPLTYPE RGB_ALPHA\n\
    ENDHDR\n\
";

pub fn convert_pnm2flif(src_path: &Path, dst_path: &Path) -> io::Result<()> {
    let src = load_raw_pnm(src_path)?;
    let mut rgba_buf = vec![0u8; WIDTH*HEIGHT];
    raw2rgba_flip(&src, &mut rgba_buf);

    let mut file = tempfile::NamedTempFile::new()?;
    file.write_all(PAM_HEADER)?;
    file.write_all(&rgba_buf)?;
    file.flush()?;

    let status = Command::new("flif")
        .arg("-eKNB")
        .arg("-E100")
        .arg("-Q100")
        .arg("--overwrite")
        .arg(file.path())
        .arg(dst_path)
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .status()
        .expect("failed to execute process");
    if !status.success() {
        let err_msg = format!("flif failure: {}", src_path.display());
        Err(io::Error::new(io::ErrorKind::Other, err_msg))?;
    }
    file.close()?;
    Ok(())
}

fn main() -> io::Result<()> {
    let args = Cli::from_args();

    fs::create_dir_all(&args.dst_dir)?;
    let pnm_ext = std::ffi::OsStr::new("pnm");

    let mut tasks: Vec<(PathBuf, PathBuf)> = Default::default();
    for entry in std::fs::read_dir(&args.src_dir)? {
        let entry = entry?;
        let src_path = entry.path();
        let ft = entry.file_type()?;
        if !ft.is_file() || src_path.extension() != Some(pnm_ext) { continue; }
        let dst_fname = Path::new(src_path.file_name().unwrap().into())
            .with_extension("flif");
        let dst_path = args.dst_dir.join(dst_fname);
        tasks.push((src_path, dst_path));
    }

    println!("Processing: {}", args.src_dir.display());
    let bar = ProgressBar::new(tasks.len() as u64);
    bar.set_style(ProgressStyle::default_bar().template(PBAR_TEMPLATE));
    tasks.par_iter()
        .progress_with(bar)
        .for_each(|(src_path, dst_path)| {
            convert_pnm2flif(src_path, dst_path).expect("conversion failed");
        });

    Ok(())
}
