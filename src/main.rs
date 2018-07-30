#![feature(exact_chunks)]
#![feature(extern_types)]
extern crate pbr;
#[macro_use] extern crate structopt;
extern crate num_cpus;
extern crate png;
extern crate flif;
extern crate memmap;

use structopt::StructOpt;

//mod ffi;
//mod flif;
mod opt;
mod utils;
mod convert;
mod convert_stereo;
mod bayer;

use opt::OscarOpt;

fn main() {
    let opt = OscarOpt::from_args();
    let res = match opt {
        OscarOpt::Convert { opt } => convert::convert(opt),
        OscarOpt::ConvertStereo { opt } => convert_stereo::convert(opt),
    };
    match res {
        Ok(()) => (),
        Err(err) => println!("Error: {:?}", err),
    }
}
