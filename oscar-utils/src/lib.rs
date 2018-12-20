pub mod conversions;
pub mod load_frames;
mod bayer;

pub use self::bayer::bggr_bayer;

pub const WIDTH: usize = 2448;
pub const HEIGHT: usize = 2048;

pub const PBAR_TEMPLATE: &str = "\
    {wide_bar} {percent:>3}% {pos:>7}/{len} \
    Elapsed: {elapsed_precise} ETA: {eta_precise}\
";

