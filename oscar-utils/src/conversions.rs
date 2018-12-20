use super::{WIDTH, HEIGHT};

/// Flips frame and converts from raw Bayer to RGBA fromat
pub fn raw2rgba_flip(src: &[u8], dst: &mut [u8]) {
    assert_eq!(src.len(), WIDTH*HEIGHT);
    assert_eq!(dst.len(), WIDTH*HEIGHT);

    for y in 0..HEIGHT/2 {
        for x in 0..WIDTH/2 {
            let rgba_pos = 4*(y*WIDTH/2 + x);
            let raw_pos = (HEIGHT - 2*y - 1)*WIDTH + (WIDTH - 2*x - 1);
            unsafe {
                let b = *src.get_unchecked(raw_pos);
                let g1 = *src.get_unchecked(raw_pos - 1);
                let g2 = *src.get_unchecked(raw_pos - WIDTH);
                let r = *src.get_unchecked(raw_pos - 1 - WIDTH);
                *dst.get_unchecked_mut(rgba_pos + 0) = r;
                *dst.get_unchecked_mut(rgba_pos + 1) = g1;
                *dst.get_unchecked_mut(rgba_pos + 2) = b;
                *dst.get_unchecked_mut(rgba_pos + 3) =
                    g2.wrapping_sub(g1).wrapping_add(0x80);
            }
        }
    }
}

/// Converts frame from RGBA to raw Bayer fromat (but does no perform flipping!)
pub fn rgba2raw(src: &[u8], dst: &mut [u8]) {
    assert_eq!(src.len(), WIDTH*HEIGHT);
    assert_eq!(dst.len(), WIDTH*HEIGHT);

    for y in 0..HEIGHT/2 {
        for x in 0..WIDTH/2 {
            let rgba_pos = 4*(y*WIDTH/2 + x);
            let raw_pos = 2*WIDTH*y + 2*x;
            unsafe {
                let r = *src.get_unchecked(rgba_pos + 0);
                let g1 = *src.get_unchecked(rgba_pos + 1);
                let b = *src.get_unchecked(rgba_pos + 2);
                let delta = *src.get_unchecked(rgba_pos + 3);
                let g2 = delta.wrapping_add(g1).wrapping_sub(0x80);

                *dst.get_unchecked_mut(raw_pos + 0) = b;
                *dst.get_unchecked_mut(raw_pos + 1) = g1;
                *dst.get_unchecked_mut(raw_pos + WIDTH) = g2;
                *dst.get_unchecked_mut(raw_pos + WIDTH + 1) = r;
            }
        }
    }
}

/// Performs in-place horizontal flip of raw Bayer image
pub fn raw_flip(buf: &mut [u8]) {
    assert_eq!(buf.len(), WIDTH*HEIGHT);
    assert_eq!(HEIGHT % 2, 0);
    for y in 0..HEIGHT/2 {
        for x in 0..WIDTH {
            let pos1 = y*WIDTH + x;
            let pos2 = (HEIGHT - y - 1)*WIDTH + (WIDTH - x - 1);
            unsafe {
                let t = *buf.get_unchecked(pos1);
                *buf.get_unchecked_mut(pos1) = *buf.get_unchecked(pos2);
                *buf.get_unchecked_mut(pos2) = t;
            }
        }
    }
}
