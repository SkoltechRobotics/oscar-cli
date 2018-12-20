use structopt::StructOpt;
use std::path::PathBuf;

#[derive(StructOpt)]
#[structopt(name = "convert",
    about = "Tool for converting OS:Car project images")]
pub enum Cli {
    /// Convert images from single camera
    #[structopt(name = "mono")]
    Mono {
        #[structopt(flatten)]
        opt: ConvertOpt
    },
    /// Join left and right images into a single one. Left image names will be
    /// used for output files.
    #[structopt(name = "stereo")]
    Stereo {
        #[structopt(flatten)]
        opt: ConvertStereoOpt,
    },
}

#[derive(StructOpt, Copy, Clone, Eq, PartialEq)]
pub enum Format {
    Pnm,
    Png,
    Jpeg,
}

impl ::std::str::FromStr for Format {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pnm" => Ok(Format::Pnm),
            "png" => Ok(Format::Png),
            "jpeg" => Ok(Format::Jpeg),
            _ => Err("unexpected format")
        }
    }
}

impl Default for Format {
    fn default() -> Self {
        Format::Pnm
    }
}

fn parse_scale(s: &str) -> Result<u8, String> {
    let res = s.parse().map_err(|err| format!("{}", err))?;
    match res {
        1 | 2 | 4 | 8 | 16 => Ok(res),
        _ => Err("Unsupported scale factor".to_string()),
    }
}

#[derive(StructOpt, Clone)]
pub struct ConvertOpt {
    #[structopt(flatten)]
    pub format: FormatOpt,
    /// Skip first N images
    #[structopt(short = "n", default_value = "0")]
    pub skip: u32,
    /// Input directory
    #[structopt(parse(from_os_str))]
    pub input: PathBuf,
    /// Output directory
    #[structopt(parse(from_os_str))]
    pub output: PathBuf,
}

#[derive(StructOpt, Clone)]
pub struct ConvertStereoOpt {
    #[structopt(flatten)]
    pub format: FormatOpt,
    /// Ignore partial pairs (with only left ot right image)
    #[structopt(long = "ignore_partial")]
    pub ignore_partial: bool,
    /// Ignore empty pairs (without left and right image)
    #[structopt(long = "ignore_empty")]
    pub ignore_empty: bool,
    /// Skip first N pairs (including partial and full)
    #[structopt(short = "n", default_value = "0")]
    pub skip: u32,
    /// Input directory
    #[structopt(parse(from_os_str))]
    pub input: PathBuf,
    /// Output directory
    #[structopt(parse(from_os_str))]
    pub output: PathBuf,
}

#[derive(StructOpt, Clone)]
pub struct FormatOpt {
    /// Apply bi-linear demosaicing
    #[structopt(short = "d")]
    pub demosaic: bool,
    /// Apply histogram equalization filter
    #[structopt(long = "histeq")]
    pub histeq: bool,
    /// Format of output files. Supported formats: pnm, png, jpeg.
    #[structopt(short = "f", parse(try_from_str), default_value = "png")]
    pub format: Format,
    /// Downscale images using given scale factor. Can be used only with enabled
    /// demosaicing. Accepted values: 1, 2, 4, 8, 16.
    #[structopt(short = "s", default_value = "1", parse(try_from_str="parse_scale"))]
    pub scale: u8,
    /// Encoding quality (usable only with the format equal to jpeg)
    #[structopt(short = "q", default_value = "90")]
    pub quality: u8,
}
