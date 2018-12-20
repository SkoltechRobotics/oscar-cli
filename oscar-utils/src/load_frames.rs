use std::path::Path;
use std::fs::File;
use std::io::{self, Read};

use super::{WIDTH, HEIGHT};

pub fn load_raw_pnm(path: &Path) -> io::Result<Box<[u8]>> {
    let mut f = File::open(path)?;
    let mut buf = [0; 17];
    f.read_exact(&mut buf)?;
    assert_eq!(&buf, b"P5\n2448 2048\n255\n");
    let mut buf = vec![0u8; WIDTH*HEIGHT];
    f.read_exact(&mut buf)?;
    Ok(buf.into_boxed_slice())
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
            channels: flif::colors::ColorSpace::RGBA,
        } if width == wt && height == ht => image.get_raw_pixels(),
        _ => Err(io::Error::new(io::ErrorKind::InvalidData,
            format!("unexpected image properites: {:?}", header)))?,
    };
    let mut raw = vec![0u8; WIDTH*HEIGHT].into_boxed_slice();
    crate::conversions::rgba2raw(&rgba, &mut raw);
    Ok(raw)
}