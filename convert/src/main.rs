use structopt::StructOpt;

mod cli;
mod utils;
mod mono;
mod mono_tar;
mod stereo;

use crate::cli::Cli;

fn main() {
    let opt = Cli::from_args();
    let res = match opt {
        Cli::Mono { opt } => if opt.input.ends_with(".tar") {
            mono_tar::convert(opt)
        } else {
            mono::convert(opt)
        },
        Cli::Stereo { opt } => stereo::convert(opt),
    };
    match res {
        Ok(()) => (),
        Err(err) => println!("Error: {:?}", err),
    }
}
