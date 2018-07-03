#![feature(test)]
#![feature(exact_chunks)]
extern crate test;

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


#[inline(never)]
fn get_img() -> Vec<u8> {
    vec![0u8; 3*2448*2048]
}

#[inline(never)]
fn get_img_gray() -> Vec<u8> {
    vec![0u8; 2448*2048]
}

#[bench]
fn bench_resize(b: &mut test::Bencher) {
    let img = get_img();
    b.iter(|| {
        let res = resize(&img, 2448, 2048, 2);
        test::black_box(res);
    });
}

fn get_idx(x: usize, y: usize, width: usize) -> usize {
    y*width + x
}

unsafe fn get(data: &[u8], x: usize, y: usize, width: usize) -> u16 {
    let idx = get_idx(x, y, width);
    debug_assert!(data.len() < idx);
    debug_assert!(x < width);
    *data.get_unchecked(idx) as u16
}

unsafe fn set(data: &mut [u8], x: usize, y: usize, col: u8, width: usize, val: u16) {
    let idx = 3*get_idx(x, y, width) + col as usize;
    debug_assert!(data.len() < idx);
    debug_assert!(x < width);
    *(data.get_unchecked_mut(idx)) = val as u8;
}

unsafe fn first_row(buf: &mut [u8], data: &[u8], x: usize, w: usize, h: usize) {
    debug_assert!(buf.len() == 3*w*h);
    debug_assert!(data.len() == w*h);
    debug_assert!(x < w);

    let y = 0;

    let g3 = get(data, x-1, y, w);
    let g5 = get(data, x+1, y, w);
    let g8 = get(data, x  , y+1, w);
    let g10 = get(data, x+2, y+1, w);
    let g12 = get(data, x+1, y+2, w);

    set(buf, x, y, 1, w, (g3+g5+g8)/3);
    set(buf, x+1, y, 1, w, g5);
    set(buf, x, y+1, 1, w, g8);
    set(buf, x+1, y+1, 1, w, (g5+g8+g10+g12)/4);

    let r7 = get(data, x-1, y+1, w);
    let r9 = get(data, x+1, y+1, w);

    set(buf, x, y, 0, w, (r7+r9)/2);
    set(buf, x+1, y, 0, w, r9);
    set(buf, x, y+1, 0, w, (r7+r9)/2);
    set(buf, x+1, y+1, 0, w, r9);

    let b4 = get(data, x  , y, w);
    let b6 = get(data, x+2, y, w);
    let b11 = get(data, x, y+2, w);
    let b13 = get(data, x+2, y+2, w);

    set(buf, x, y, 2, w, b4);
    set(buf, x+1, y, 2, w, (b4+b6)/2);
    set(buf, x, y+1, 2, w, (b4+b11)/2);
    set(buf, x+1, y+1, 2, w, (b4+b6+b11+b13)/4);
}

unsafe fn last_row(buf: &mut [u8], data: &[u8], x: usize, w: usize, h: usize) {
    debug_assert!(buf.len() == 3*w*h);
    debug_assert!(data.len() == w*h);
    debug_assert!(x < w);

    let y = h - 1;

    let g1 = get(data, x  , y-1, w);
    let g3 = get(data, x-1, y, w);
    let g5 = get(data, x+1, y, w);
    let g8 = get(data, x  , y+1, w);
    let g10 = get(data, x+2, y+1, w);

    set(buf, x, y, 1, w, (g1+g3+g5+g8)/4);
    set(buf, x+1, y, 1, w, g5);
    set(buf, x, y+1, 1, w, g8);
    set(buf, x+1, y+1, 1, w, (g5+g8+g10)/3);

    let r0 = get(data, x-1, y-1, w);
    let r2 = get(data, x+1, y-1, w);
    let r7 = get(data, x-1, y+1, w);
    let r9 = get(data, x+1, y+1, w);

    set(buf, x, y, 0, w, (r0+r2+r7+r9)/4);
    set(buf, x+1, y, 0, w, (r2+r9)/2);
    set(buf, x, y+1, 0, w, (r7+r9)/2);
    set(buf, x+1, y+1, 0, w, r9);

    let b4 = get(data, x  , y, w);
    let b6 = get(data, x+2, y, w);

    set(buf, x, y, 2, w, b4);
    set(buf, x+1, y, 2, w, (b4+b6)/2);
    set(buf, x, y+1, 2, w, b4);
    set(buf, x+1, y+1, 2, w, (b4+b6)/2);
}

unsafe fn first_column(buf: &mut [u8], data: &[u8], y: usize, w: usize, h: usize) {
    debug_assert!(buf.len() == 3*w*h);
    debug_assert!(data.len() == w*h);
    debug_assert!(y < h);

    let x = 0;

    let g1 = get(data, x  , y-1, w);
    let g5 = get(data, x+1, y, w);
    let g8 = get(data, x  , y+1, w);
    let g10 = get(data, x+2, y+1, w);
    let g12 = get(data, x+1, y+2, w);

    set(buf, x, y, 1, w, (g1+g5+g8)/3);
    set(buf, x+1, y, 1, w, g5);
    set(buf, x, y+1, 1, w, g8);
    set(buf, x+1, y+1, 1, w, (g5+g8+g10+g12)/4);

    let r2 = get(data, x+1, y-1, w);
    let r9 = get(data, x+1, y+1, w);

    set(buf, x, y, 0, w, (r2+r9)/2);
    set(buf, x+1, y, 0, w, (r2+r9)/2);
    set(buf, x, y+1, 0, w, r9);
    set(buf, x+1, y+1, 0, w, r9);

    let b4 = get(data, x  , y, w);
    let b6 = get(data, x+2, y, w);
    let b11 = get(data, x, y+2, w);
    let b13 = get(data, x+2, y+2, w);

    set(buf, x, y, 2, w, b4);
    set(buf, x+1, y, 2, w, (b4+b6)/2);
    set(buf, x, y+1, 2, w, (b4+b11)/2);
    set(buf, x+1, y+1, 2, w, (b4+b6+b11+b13)/4);
}

unsafe fn last_column(buf: &mut [u8], data: &[u8], y: usize, w: usize, h: usize) {
    debug_assert!(buf.len() == 3*w*h);
    debug_assert!(data.len() == w*h);
    debug_assert!(y < h);

    let x = w - 1;

    let g1 = get(data, x  , y-1, w);
    let g3 = get(data, x-1, y, w);
    let g5 = get(data, x+1, y, w);
    let g8 = get(data, x  , y+1, w);
    let g12 = get(data, x+1, y+2, w);

    set(buf, x, y, 1, w, (g1+g3+g5+g8)/4);
    set(buf, x+1, y, 1, w, g5);
    set(buf, x, y+1, 1, w, g8);
    set(buf, x+1, y+1, 1, w, (g5+g8+g12)/3);

    let r0 = get(data, x-1, y-1, w);
    let r2 = get(data, x+1, y-1, w);
    let r7 = get(data, x-1, y+1, w);
    let r9 = get(data, x+1, y+1, w);

    set(buf, x, y, 0, w, (r0+r2+r7+r9)/4);
    set(buf, x+1, y, 0, w, (r2+r9)/2);
    set(buf, x, y+1, 0, w, (r7+r9)/2);
    set(buf, x+1, y+1, 0, w, r9);

    let b4 = get(data, x  , y, w);
    let b11 = get(data, x, y+2, w);

    set(buf, x, y, 2, w, b4);
    set(buf, x+1, y, 2, w, b4);
    set(buf, x, y+1, 2, w, (b4+b11)/2);
    set(buf, x+1, y+1, 2, w, (b4+b11)/2);
}

unsafe fn top_left_corner(buf: &mut [u8], data: &[u8], w: usize, h: usize) {
    debug_assert!(buf.len() == 3*w*h);
    debug_assert!(data.len() == w*h);

    let x = 0;
    let y = 0;
    let g5 = get(data, x+1, y, w);
    let g8 = get(data, x  , y+1, w);
    let g10 = get(data, x+2, y+1, w);
    let g12 = get(data, x+1, y+2, w);

    set(buf, x, y, 1, w, (g5+g8)/2);
    set(buf, x+1, y, 1, w, g5);
    set(buf, x, y+1, 1, w, g8);
    set(buf, x+1, y+1, 1, w, (g5+g8+g10+g12)/4);

    let r9 = get(data, x+1, y+1, w);

    set(buf, x, y, 0, w, r9);
    set(buf, x+1, y, 0, w, r9);
    set(buf, x, y+1, 0, w, r9);
    set(buf, x+1, y+1, 0, w, r9);

    let b4 = get(data, x  , y, w);
    let b6 = get(data, x+2, y, w);
    let b11 = get(data, x, y+2, w);
    let b13 = get(data, x+2, y+2, w);

    set(buf, x, y, 2, w, b4);
    set(buf, x+1, y, 2, w, (b4+b6)/2);
    set(buf, x, y+1, 2, w, (b4+b11)/2);
    set(buf, x+1, y+1, 2, w, (b4+b6+b11+b13)/4);
}


unsafe fn top_right_corner(buf: &mut [u8], data: &[u8], w: usize, h: usize) {
    debug_assert!(buf.len() == 3*w*h);
    debug_assert!(data.len() == w*h);

    let x = w - 1;
    let y = 0;
    let g3 = get(data, x-1, y, w);
    let g5 = get(data, x+1, y, w);
    let g8 = get(data, x  , y+1, w);
    let g12 = get(data, x+1, y+2, w);

    set(buf, x, y, 1, w, (g3+g5+g8)/3);
    set(buf, x+1, y, 1, w, g5);
    set(buf, x, y+1, 1, w, g8);
    set(buf, x+1, y+1, 1, w, (g5+g8+g12)/3);

    let r7 = get(data, x-1, y+1, w);
    let r9 = get(data, x+1, y+1, w);

    set(buf, x, y, 0, w, (r7+r9)/2);
    set(buf, x+1, y, 0, w, r9);
    set(buf, x, y+1, 0, w, (r7+r9)/2);
    set(buf, x+1, y+1, 0, w, r9);

    let b4 = get(data, x  , y, w);
    let b11 = get(data, x, y+2, w);

    set(buf, x, y, 2, w, b4);
    set(buf, x+1, y, 2, w, b4);
    set(buf, x, y+1, 2, w, (b4+b11)/2);
    set(buf, x+1, y+1, 2, w, (b4+b11)/2);
}


unsafe fn bottom_left_corner(buf: &mut [u8], data: &[u8], w: usize, h: usize) {
    debug_assert!(buf.len() == 3*w*h);
    debug_assert!(data.len() == w*h);

    let x = 0;
    let y = h - 1;

    let g1 = get(data, x  , y-1, w);
    let g5 = get(data, x+1, y, w);
    let g8 = get(data, x  , y+1, w);
    let g10 = get(data, x+2, y+1, w);

    set(buf, x, y, 1, w, (g1+g5+g8)/3);
    set(buf, x+1, y, 1, w, g5);
    set(buf, x, y+1, 1, w, g8);
    set(buf, x+1, y+1, 1, w, (g5+g8+g10)/3);

    let r2 = get(data, x+1, y-1, w);
    let r9 = get(data, x+1, y+1, w);

    set(buf, x, y, 0, w, (r2+r9)/2);
    set(buf, x+1, y, 0, w, (r2+r9)/2);
    set(buf, x, y+1, 0, w, r9);
    set(buf, x+1, y+1, 0, w, r9);

    let b4 = get(data, x  , y, w);
    let b6 = get(data, x+2, y, w);

    set(buf, x, y, 2, w, b4);
    set(buf, x+1, y, 2, w, (b4+b6)/2);
    set(buf, x, y+1, 2, w, b4);
    set(buf, x+1, y+1, 2, w, (b4+b6)/2);
}


unsafe fn bottom_right_corner(buf: &mut [u8], data: &[u8], w: usize, h: usize) {
    debug_assert!(buf.len() == 3*w*h);
    debug_assert!(data.len() == w*h);

    let x = w - 1;
    let y = h - 1;

    let g1 = get(data, x  , y-1, w);
    let g3 = get(data, x-1, y, w);
    let g5 = get(data, x+1, y, w);
    let g8 = get(data, x  , y+1, w);

    set(buf, x, y, 1, w, (g1+g3+g5+g8)/4);
    set(buf, x+1, y, 1, w, g5);
    set(buf, x, y+1, 1, w, g8);
    set(buf, x+1, y+1, 1, w, (g5+g8)/2);

    let r0 = get(data, x-1, y-1, w);
    let r2 = get(data, x+1, y-1, w);
    let r7 = get(data, x-1, y+1, w);
    let r9 = get(data, x+1, y+1, w);

    set(buf, x, y, 0, w, (r0+r2+r7+r9)/4);
    set(buf, x+1, y, 0, w, (r2+r9)/2);
    set(buf, x, y+1, 0, w, (r7+r9)/2);
    set(buf, x+1, y+1, 0, w, r9);

    let b4 = get(data, x  , y, w);

    set(buf, x, y, 2, w, b4);
    set(buf, x+1, y, 2, w, b4);
    set(buf, x, y+1, 2, w, b4);
    set(buf, x+1, y+1, 2, w, b4);
}

// pixel indexes (pixel number 4 has coordinates x, y):
// 0  1  2
// 3  4  5  6
// 7  8  9  10
//    11 12 13
// which represent the following colors (we work with BGGR pattern):
// r  g  r
// g  b  g  b
// r  g  r  g
//    b  g  b
unsafe fn core(buf: &mut [u8], data: &[u8], x: usize, y: usize, w: usize, h: usize) {
    debug_assert!(buf.len() == 3*w*h);
    debug_assert!(data.len() == w*h);
    debug_assert!(x > 0);
    debug_assert!(y > 0);
    debug_assert!(x < w);
    debug_assert!(y < h);

    let g1 = get(data, x  , y-1, w);
    let g3 = get(data, x-1, y, w);
    let g5 = get(data, x+1, y, w);
    let g8 = get(data, x  , y+1, w);
    let g10 = get(data, x+2, y+1, w);
    let g12 = get(data, x+1, y+2, w);

    set(buf, x, y, 1, w, (g1+g3+g5+g8)/4);
    set(buf, x+1, y, 1, w, g5);
    set(buf, x, y+1, 1, w, g8);
    set(buf, x+1, y+1, 1, w, (g5+g8+g10+g12)/4);

    let r0 = get(data, x-1, y-1, w);
    let r2 = get(data, x+1, y-1, w);
    let r7 = get(data, x-1, y+1, w);
    let r9 = get(data, x+1, y+1, w);

    set(buf, x, y, 0, w, (r0+r2+r7+r9)/4);
    set(buf, x+1, y, 0, w, (r2+r9)/2);
    set(buf, x, y+1, 0, w, (r7+r9)/2);
    set(buf, x+1, y+1, 0, w, r9);

    let b4 = get(data, x  , y, w);
    let b6 = get(data, x+2, y, w);
    let b11 = get(data, x, y+2, w);
    let b13 = get(data, x+2, y+2, w);

    set(buf, x, y, 2, w, b4);
    set(buf, x+1, y, 2, w, (b4+b6)/2);
    set(buf, x, y+1, 2, w, (b4+b11)/2);
    set(buf, x+1, y+1, 2, w, (b4+b6+b11+b13)/4);
}

fn bggr_bayer(data: &[u8], width: usize, height: usize) -> Vec<u8> {
    assert_eq!(data.len(), width*height);
    assert_eq!(width % 2, 0);
    assert_eq!(height % 2, 0);
    let mut buf = vec![0u8; 3*width*height];

    unsafe {
        top_left_corner(&mut buf, data, width, height);
        for x in (1..width/2 - 1).map(|v| 2*v) {
            first_row(&mut buf, data, x, width, height);
        }
        top_right_corner(&mut buf, data, width, height);

        for y in (1..height/2 - 1).map(|v| 2*v) {
            first_column(&mut buf, data, y, width, height);
            for x in (1..width/2 - 1).map(|v| 2*v) {
                core(&mut buf, data, x, y, width, height);
            }
            last_column(&mut buf, data, y, width, height);
        }

        bottom_left_corner(&mut buf, data, width, height);
        for x in (1..width/2 - 1).map(|v| 2*v) {
            last_row(&mut buf, data, x, width, height);
        }
        bottom_right_corner(&mut buf, data, width, height);
    }

    buf
}

#[bench]
fn bench_bayer(b: &mut test::Bencher) {
    let img = get_img_gray();
    b.iter(|| {
        test::black_box(bggr_bayer(&img, 2448, 2048));
    });
}