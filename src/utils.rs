use std::path::Path;
use std::{io, fs};
use std::io::{Read, Write};

use {bayer, png, flif};
use png::HasParameters;
use opt::{Format, FormatOpt};

pub fn read_flif(path: &Path) -> io::Result<Vec<u8>> {
    let file_size = fs::metadata(path)?.len();
    let mut data = Vec::with_capacity(file_size as usize);
    fs::File::open(path)?.read_to_end(&mut data)?;
    let dec = flif::FlifDecoder::new(&data)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData,
            "FLIF decoder error".to_string(),
        ))?;

    let (width, height) = (dec.width(), dec.height());
    let (depth, frames) = (dec.depth(), dec.frames());
    let channels = dec.channels();
    let img_data = match (width, height, depth, channels, frames) {
        (2448, 2048, 8, 1, 1) => dec.get_image_data(0),
        _ => Err(io::Error::new(io::ErrorKind::InvalidData,
            "unexpected image properites".to_string(),
        ))?,
    };
    // get sum of first 128 pixels to check for erroneous decoding
    // see: https://github.com/FLIF-hub/FLIF/issues/517
    let sum100 = img_data[..128].iter().fold(0u32, |a, v| a + *v as u32);
    if sum100 == 128*15 || sum100 == 0 { return read_flif(path); }

    Ok(img_data)
}

pub fn save_img(
    name: &str, mut data: Vec<u8>, opt: &FormatOpt, out_dir: &Path,
    width: u32, height: u32,
) -> io::Result<()> {
    assert_eq!(data.len(), (width*height) as usize);
    let is_color = if opt.demosaic {
        data = bayer::bggr_bayer(&data, width as usize, height as usize);
        true
    } else {
        false
    };
    let mut width = width;
    let mut height = height;
    if opt.scale != 1 {
        data = resize(&data, width, height, opt.scale);
        width /= opt.scale as u32;
        height /= opt.scale as u32;
    }
    let mut path = out_dir.to_path_buf();
    path.push(name);
    let flag = path.set_extension(match opt.format {
        Format::Pnm => "pnm",
        Format::Png => "png",
    });
    assert!(flag, "extension set check");
    match opt.format {
        Format::Pnm => save_pnm(&path, &data, width, height, is_color),
        Format::Png => save_png(&path, &data, width, height, is_color),
    }
}


fn save_pnm(
    path: &Path, data: &[u8], width: u32, height: u32, is_color: bool,
) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    let header = if is_color {
        assert_eq!(3*width*height, data.len() as u32);
        format!("P6\n{} {}\n255\n", width, height)
    } else {
        assert_eq!(width*height, data.len() as u32);
        format!("P5\n{} {}\n255\n", width, height)
    };
    file.write_all(header.as_bytes())?;
    file.write_all(data)?;
    Ok(())
}


fn save_png(
    path: &Path, data: &[u8], width: u32, height: u32, is_color: bool,
) -> io::Result<()> {
    let target_len = if is_color { 3*width*height } else { width*height };
    assert_eq!(data.len() as u32, target_len);

    let file = fs::File::create(path)?;
    let writer = io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, width, height);

    let color = match is_color {
        true => png::ColorType::RGB,
        false => png::ColorType::Grayscale,
    };

    encoder.set(color).set(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(data)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
}

fn resize(data: &[u8], width: u32, height: u32, scale: u8) -> Vec<u8> {
    assert_eq!(data.len() as u32, 3*width*height);
    assert!(scale == 2 || scale == 4 || scale == 8 || scale == 16 );
    // scale = 2^factor
    let factor = 7 - scale.leading_zeros();
    let w = (width as usize)/(scale as usize);
    let h = (height as usize)/(scale as usize);
    let mut buf = vec![0u16; 3*w*h];

    for (y, row) in data.exact_chunks(3*width as usize).enumerate() {
        let i0 = 3*w*(y>>factor);
        for (x, pix) in row.exact_chunks(3).enumerate() {
            let idx = i0 + 3*(x>>factor);
            unsafe {
                *(buf.get_unchecked_mut(idx + 0)) += pix[0] as u16;
                *(buf.get_unchecked_mut(idx + 1)) += pix[1] as u16;
                *(buf.get_unchecked_mut(idx + 2)) += pix[2] as u16;
            }
        }
    }

    buf.iter().map(|v| (v >> (factor*2)) as u8).collect()
}
