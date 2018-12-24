use std::{io, path::PathBuf};
use structopt::StructOpt;

mod verify;
mod convert;

const PAM_HEADER: &[u8] = b"\
    P7\n\
    WIDTH 1224\n\
    HEIGHT 1024\n\
    DEPTH 4\n\
    MAXVAL 255\n\
    TUPLTYPE RGB_ALPHA\n\
    ENDHDR\n\
";

#[derive(StructOpt)]
#[structopt(
    name = "pnm2flif",
    about = "Convert raw Bayer PNM frames to RGBA FLIF.")]
struct Cli {
    /// Do not convert and instead verify content equality of PNM and
    /// RGBA FLIF frames
    #[structopt(long = "verify")]
    pub verify: bool,
    /// Path to the raw PNM frames directory
    #[structopt(parse(from_os_str))]
    pnm_dir: PathBuf,
    /// Path to the RGBA FLIF frames directory
    #[structopt(parse(from_os_str))]
    flif_dir: PathBuf,
}

fn main() -> io::Result<()> {
    let args = Cli::from_args();
    if args.verify {
        verify::verify(args)
    } else {
        convert::convert(args)
    }
}
