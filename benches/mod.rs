#![feature(test)]
extern crate test;

use oscar_utils;
use oscar_utils::{bggr_bayer, WIDTH, HEIGHT};
use oscar_utils::conversions::{rgba2raw, raw2rgba_flip, raw_flip};

#[inline(never)]
fn get_buf() -> Vec<u8> {
    vec![7u8; WIDTH*HEIGHT]
}

#[bench]
fn bench_bggr_bayer(b: &mut test::Bencher) {
    let src = get_buf();
    b.iter(|| {
        let res = bggr_bayer(&src, WIDTH, HEIGHT);
        test::black_box(res);
    });
}

#[bench]
fn bench_raw2rgba(b: &mut test::Bencher) {
    let src = get_buf();
    let mut dst = get_buf();
    b.iter(|| {
        raw2rgba_flip(&src, &mut dst);
        test::black_box(&dst);
    });
}

#[bench]
fn bench_rgba2raw(b: &mut test::Bencher) {
    let src = get_buf();
    let mut dst = get_buf();
    b.iter(|| {
        rgba2raw(&src, &mut dst);
        test::black_box(&dst);
    });
}

#[bench]
fn bench_flip(b: &mut test::Bencher) {
    let mut buf = get_buf();
    b.iter(|| {
        raw_flip(&mut buf);
        test::black_box(&buf);
    });
}
