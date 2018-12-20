use structopt::StructOpt;

mod cli;
mod utils;
mod mono;
mod stereo;

use self::cli::Cli;

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
