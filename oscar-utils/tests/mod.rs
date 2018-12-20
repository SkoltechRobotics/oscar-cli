
use oscar_utils::{WIDTH, HEIGHT};
use oscar_utils::conversions::{rgba2raw, raw2rgba_flip, raw_flip};

fn test_image() -> Vec<u8> {
    (0..WIDTH*HEIGHT).map(|n| (n % 256) as u8).collect()
}

#[test]
fn test_flip() {
    let orig = test_image();
    let mut buf = orig.clone();
    raw_flip(&mut buf);
    assert!(buf != orig);
    raw_flip(&mut buf);
    assert!(buf == orig);
}

#[test]
fn test_conversions() {
    let orig = test_image();
    let mut buf1 = vec![0u8; WIDTH*HEIGHT];
    let mut buf2 = vec![0u8; WIDTH*HEIGHT];
    raw2rgba_flip(&orig, &mut buf1);
    rgba2raw(&buf1, &mut buf2);
    raw_flip(&mut buf2);
    assert_eq!(orig, buf2);
}
