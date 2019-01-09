use std::process::{Command, Stdio};
use std::path::Path;
use std::io;

use indicatif::{ProgressBar, ProgressStyle, ParallelProgressIterator};
use rayon::iter::{ParallelIterator, IntoParallelRefIterator};

use oscar_utils::{PBAR_TEMPLATE, WIDTH, HEIGHT};
use oscar_utils::conversions::{raw_flip, rgba2raw};
use oscar_utils::load_frames::{load_raw_pnm, load_flif};

use super::PAM_HEADER;

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


fn cpp_flif_load(path: &Path) -> io::Result<Box<[u8]>> {
    let f = tempfile::Builder::new()
        .suffix(".pam")
        .tempfile()?;

    let status = Command::new("flif")
        .arg("-d")
        .arg("--overwrite")
        .arg(path)
        .arg(f.path())
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .status()
        .expect("failed to execute process");
    if !status.success() {
        let err_msg = format!("flif failure: {}", path.display());
        Err(io::Error::new(io::ErrorKind::Other, err_msg))?;
    }
    let mmap = unsafe { memmap::Mmap::map(f.as_file())? };
    let (header, image) = mmap.split_at(PAM_HEADER.len());
    if header != PAM_HEADER || image.len() != WIDTH*HEIGHT {
        Err(io::Error::new(io::ErrorKind::Other,
            "invalid PAM frame".to_string()))?;
    }
    let mut raw = vec![0u8; WIDTH*HEIGHT];
    rgba2raw(image, &mut raw);
    Ok(raw.into_boxed_slice())
}

#[derive(Debug, Copy, Clone)]
struct CompResult {
    /// flif CLI tool comparison result
    cpp: bool,
    /// Rust library comparison result
    rs: bool,
}

fn compare(fname: &str, flif_dir: &Path, pnm_dir: &Path) -> io::Result<CompResult> {
    let flif_path = flif_dir.join(&fname).with_extension("flif");
    let pnm_path = pnm_dir.join(&fname).with_extension("pnm");

    let pnm_frame = load_raw_pnm(&pnm_path)?;
    let mut rs_flif_frame = load_flif(&flif_path)?;
    raw_flip(&mut rs_flif_frame);

    let mut cpp_flif_frame = cpp_flif_load(&flif_path)?;
    raw_flip(&mut cpp_flif_frame);

    let res = CompResult {
        cpp: &pnm_frame[..] == &cpp_flif_frame[..],
        rs: &pnm_frame[..] == &rs_flif_frame[..],
    };
    if !res.cpp || !res.rs {
        println!("{} {:?}\n", fname, res);
    }
    Ok(res)
}

pub(crate) fn verify(args: crate::Cli) -> io::Result<()> {
    println!("Verification: {} {}",
        args.pnm_dir.display(), args.flif_dir.display());

    let flifs = get_filenames(&args.flif_dir, "flif")?;
    let pnms = get_filenames(&args.pnm_dir, "pnm")?;
    if flifs != pnms {
        panic!("extra or missing files");
    }
    let fnames = flifs;

    let bar = ProgressBar::new(fnames.len() as u64);
    bar.set_style(ProgressStyle::default_bar().template(PBAR_TEMPLATE));

    let res: Vec<(CompResult, String)> = fnames.par_iter()
        .progress_with(bar)
        .map(|fname| compare(fname, &args.flif_dir, &args.pnm_dir))
        .collect::<io::Result<Vec<CompResult>>>()?
        .iter()
        .cloned()
        .zip(fnames.into_iter())
        .filter(|(res, _)| !res.cpp || !res.rs)
        .collect();

    if res.len() == 0 {
        println!("Done. No discrepancies detected.");
    } else {
        println!("Done. Discrepancies detected: {:?}", res.len());
    }

    Ok(())
}
