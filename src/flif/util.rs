use std::{io, fs};
use std::io::Read;
use std::path::Path;

use super::FlifDecoder;

const N_TRY: usize = 5;

fn read_flif_inner(data: &[u8]) -> io::Result<Box<[u8]>> {
    let dec = FlifDecoder::new(&data)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData,
            "FLIF decoder error".to_string(),
        ))?;

    let (width, height) = (dec.width(), dec.height());
    let (depth, frames) = (dec.depth(), dec.frames());
    let channels = dec.channels();
    match (width, height, depth, channels, frames) {
        (2448, 2048, 8, 1, 1) => Ok(dec.get_image_data(0)),
        _ => Err(io::Error::new(io::ErrorKind::InvalidData,
            "unexpected image properites".to_string(),
        ))?,
    }
}

pub fn read_flif(path: &Path) -> io::Result<Box<[u8]>> {
    let file_size = fs::metadata(path)?.len();
    let mut data = Vec::with_capacity(file_size as usize);
    fs::File::open(path)?.read_to_end(&mut data)?;

    for _ in 0..N_TRY {
        let img_data = read_flif_inner(&data)?;
        // check for erroneous decoding
        // see: https://github.com/FLIF-hub/FLIF/issues/517
        let v0 = img_data[0];
        if !img_data.iter().all(|v| *v == v0) { return Ok(img_data); }
    }

    eprintln!("WARNING! after {} tries failed to correctly decode: {}",
        N_TRY, path.display());

    read_flif_inner(&data)
}