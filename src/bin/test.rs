use std::fs::File;
use std::io::{self, Read, Write};

use oscar_utils::{WIDTH, HEIGHT};
use oscar_utils::conversions::{raw_flip, raw2rgba_flip, rgba2raw};

fn main() -> Result<(), io::Error> {
    let mut f = File::open("/home/newpavlov/1.pnm")?;
    let mut buf = [0; 17];
    f.read_exact(&mut buf)?;
    assert_eq!(&buf, b"P5\n2448 2048\n255\n");
    let mut frame = vec![0u8; WIDTH*HEIGHT];
    f.read_exact(&mut frame)?;

    let mut rgba_buf = vec![0u8; WIDTH*HEIGHT];
    raw2rgba_flip(&frame, &mut rgba_buf);
    let mut f = File::create("/home/newpavlov/0rgb_flip.pnm")?;
    f.write_all(b"P6\n1224 1024\n255\n")?;
    for chunk in rgba_buf.chunks(4) {
        f.write(&chunk[..3])?;
    }

    let mut raw_buf = vec![0u8; WIDTH*HEIGHT];
    rgba2raw(&rgba_buf, &mut raw_buf);
    let mut f = File::create("/home/newpavlov/0raw_after.pnm")?;
    f.write_all(b"P5\n2448 2048\n255\n")?;
    f.write_all(&raw_buf)?;

    raw_flip(&mut frame);
    println!("flipped");
    let mut f = File::create("/home/newpavlov/0raw_flip.pnm")?;
    f.write_all(b"P5\n2448 2048\n255\n")?;
    f.write_all(&frame)?;


    Ok(())
}
