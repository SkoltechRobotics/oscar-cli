use std::path::Path;
use std::io::{self, Write};
use std::process::{Command, Stdio};

mod bayer;
pub mod conversions;
pub mod load_frames;
pub mod convert;

pub use self::bayer::bggr_bayer;

pub const WIDTH: usize = 2448;
pub const HEIGHT: usize = 2048;



const PAM_HEADER: &[u8] = b"\
    P7\n\
    WIDTH 1224\n\
    HEIGHT 1024\n\
    DEPTH 4\n\
    MAXVAL 255\n\
    TUPLTYPE RGB_ALPHA\n\
    ENDHDR\n\
";

pub const PBAR_TEMPLATE: &str = "\
    {wide_bar} {percent:>3}% {pos:>7}/{len} \
    Elapsed: {elapsed_precise} ETA: {eta_precise}\
";

// TODO: replace asserts with io::Error
pub fn convert_pnm2flif(src_path: &Path, dst_path: &Path) -> io::Result<()> {
    let src = load_frames::load_raw_pnm(src_path)?;
    let mut rgba_buf = vec![0u8; WIDTH*HEIGHT];
    conversions::raw2rgba_flip(&src, &mut rgba_buf);

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
    assert!(status.success());
    file.close()?;
    Ok(())
}
