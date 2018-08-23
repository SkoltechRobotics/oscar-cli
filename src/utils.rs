use std::path::Path;
use std::{io, fs};
use std::io::Write;

use {bayer, png};
use png::HasParameters;
use opt::{Format, FormatOpt};
use jpeg_encoder::JpegEncoder;
use jpeg_encoder;

#[cfg(feature = "libflif")]
pub use flif::read_flif;

#[cfg(not(feature = "libflif"))]
pub fn read_flif(path: &Path) -> io::Result<Box<[u8]>> {
    use memmap::Mmap;
    use flif;

    let mmap = unsafe { Mmap::map(&fs::File::open(path)?)? };
    let image = flif::Flif::decode(mmap.as_ref())
        .map_err(|err| match err {
            flif::Error::Io(err) => err,
            err => io::Error::new(io::ErrorKind::InvalidData, err)
        })?;
    let header = image.info().header;
    match header {
        flif::components::Header {
            width: 2448, height: 2048, num_frames: 1, interlaced: false,
            bytes_per_channel: flif::components::BytesPerChannel::One,
            channels: flif::colors::ColorSpace::Monochrome,
        } => Ok(image.into_raw()),
        _ => Err(io::Error::new(io::ErrorKind::InvalidData,
            format!("unexpected image properites: {:?}", header))),
    }
}

pub fn save_img(
    name: &str, mut data: Box<[u8]>, opt: &FormatOpt, out_dir: &Path,
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
        Format::Jpeg => "jpg",
    });
    assert!(flag, "extension set check");
    match opt.format {
        Format::Pnm => save_pnm(&path, &data, width, height, is_color),
        Format::Png => save_png(&path, &data, width, height, is_color),
        Format::Jpeg => save_jpeg(&path, &data, width, height, is_color, opt.quality),
    }
}

pub fn save_stereo_img(
    name: &str, mut left: Box<[u8]>, mut right: Box<[u8]>,
    opt: &FormatOpt, out_dir: &Path, width: u32, height: u32,
) -> io::Result<()> {
    assert_eq!(left.len(), (width*height) as usize);
    assert_eq!(right.len(), (width*height) as usize);
    let is_color = if opt.demosaic {
        left = bayer::bggr_bayer(&left, width as usize, height as usize);
        right = bayer::bggr_bayer(&right, width as usize, height as usize);
        true
    } else {
        false
    };
    let mut width = width;
    let mut height = height;
    if opt.scale != 1 {
        left = resize(&left, width, height, opt.scale);
        right = resize(&right, width, height, opt.scale);
        width /= opt.scale as u32;
        height /= opt.scale as u32;
    }
    let data = concat_images(left, right, width as usize, height as usize, is_color);

    let mut path = out_dir.to_path_buf();
    path.push(name);
    let flag = path.set_extension(match opt.format {
        Format::Pnm => "pnm",
        Format::Png => "png",
        Format::Jpeg => "jpg",
    });
    assert!(flag, "extension set check");
    match opt.format {
        Format::Pnm => save_pnm(&path, &data, 2*width, height, is_color),
        Format::Png => save_png(&path, &data, 2*width, height, is_color),
        Format::Jpeg => save_jpeg(&path, &data, 2*width, height, is_color, opt.quality),
    }
}

fn concat_images(
    left: Box<[u8]>, right: Box<[u8]>, w: usize, h: usize, is_color: bool
) -> Box<[u8]> {
    let w = if is_color { 3*w } else { w };
    assert_eq!(left.len(), w*h);
    assert_eq!(right.len(), w*h);
    let mut out = vec![0; 2*w*h].into_boxed_slice();
    for ((l, r), o) in left.exact_chunks(w)
        .zip(right.exact_chunks(w))
        .zip(out.exact_chunks_mut(2*w))
    {
        o[..w].copy_from_slice(l);
        o[w..].copy_from_slice(r);
    }
    out
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

fn save_jpeg(
    path: &Path, data: &[u8], width: u32, height: u32, is_color: bool, q: u8,
) -> io::Result<()> {
    let target_len = if is_color { 3*width*height } else { width*height };
    assert_eq!(data.len() as u32, target_len);

    let file = fs::File::create(path)?;
    let mut writer = io::BufWriter::new(file);
    let mut encoder = JpegEncoder::new_with_quality(&mut writer, q);

    let color = match is_color {
        true => jpeg_encoder::Color::RGB,
        false => jpeg_encoder::Color::Gray,
    };

    encoder.encode(data, width, height, color)
}

fn resize(data: &[u8], width: u32, height: u32, scale: u8) -> Box<[u8]> {
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

    buf.iter()
        .map(|v| (v >> (factor*2)) as u8)
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

#[derive(Copy, Clone, Debug)]
pub struct Timestamp {
    pub unix: u64,
    pub os: u64,
}

fn invalid_input(msg: &str, path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("{}: {}", msg, path.display())
    )
}

pub fn get_timestamp(path: &Path) -> io::Result<Timestamp> {
    if !path.is_file() {
        Err(invalid_input("expected file, but got dir", path))?;
    }
    match path.extension() {
        Some(ext) if ext == "flif" => (),
        _ => Err(invalid_input("expected file with flif extension", path))?,
    };

    let file_name = path.file_stem()
        .ok_or_else(|| invalid_input("failed to extract file stem", path))
        .and_then(|v| v.to_str().ok_or_else(||
            io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "failed to convert file name to str: {}", path.display(),
                )
            )
        ))?;

    let mut iter = file_name.split('_').map(|v| v.parse::<u64>());
    let ts = match (iter.next(), iter.next(), iter.next()) {
        (Some(Ok(unix)), Some(Ok(os)), None) => Timestamp { unix, os },
        _ => Err(invalid_input("incorrect filename pattern", path))?,
    };
    Ok(ts)
}

pub fn get_timestamps(dir_path: &Path) -> io::Result<Vec<Timestamp>> {
    let mut buf = fs::read_dir(dir_path)?
        .map(|entry| entry.and_then(|e| get_timestamp(&e.path())))
        .collect::<io::Result<Vec<Timestamp>>>()?;
    buf.sort_unstable_by(|a, b| a.os.cmp(&b.os));
    Ok(buf)
}
