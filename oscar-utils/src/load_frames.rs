use std::path::Path;
use std::fs::File;
use std::io;

use super::{WIDTH, HEIGHT};

const PNM_HEADER: &[u8] = b"P5\n2448 2048\n255\n";

pub fn load_raw_pnm(path: &Path) -> io::Result<Box<[u8]>> {
    let mmap = unsafe { memmap::Mmap::map(&File::open(path)?)? };
    let (header, image) = mmap.split_at(17);

    if header != PNM_HEADER || image.len() != WIDTH*HEIGHT {
        Err(io::Error::new(io::ErrorKind::InvalidData,
            "invalid PNM frame".to_string()))
    } else {
        Ok(image.to_vec().into_boxed_slice())
    }
}

pub fn load_flif(path: &Path) -> io::Result<Box<[u8]>> {
    let mmap = unsafe { memmap::Mmap::map(&File::open(path)?)? };
    let image = flif::Flif::decode(mmap.as_ref())
        .map_err(|err| match err {
            flif::Error::Io(err) => err,
            err => io::Error::new(io::ErrorKind::InvalidData, err)
        })?;
    let header = image.info().header;

    let wt = (WIDTH as u32)/2;
    let ht = (HEIGHT as u32)/2;
    let rgba = match header {
        flif::components::Header {
            width, height,
            num_frames: 1, interlaced: false,
            bytes_per_channel: flif::components::BytesPerChannel::One,
            channels: flif::components::ColorSpace::RGBA,
        } if width == wt && height == ht => image.into_raw(),
        _ => Err(io::Error::new(io::ErrorKind::InvalidData,
            format!("unexpected image properites: {:?}", header)))?,
    };
    let mut raw = vec![0u8; WIDTH*HEIGHT].into_boxed_slice();
    crate::conversions::rgba2raw(&rgba, &mut raw);
    Ok(raw)
}
