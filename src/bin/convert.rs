use structopt::StructOpt;

use oscar_utils::convert::cli::Cli;
use oscar_utils::convert::{stereo, mono};

fn main() {
    let opt = Cli::from_args();
    let res = match opt {
        Cli::Mono { opt } => mono::convert(opt),
        Cli::Stereo { opt } => stereo::convert(opt),
    };
    match res {
        Ok(()) => (),
        Err(err) => println!("Error: {:?}", err),
    }
}
