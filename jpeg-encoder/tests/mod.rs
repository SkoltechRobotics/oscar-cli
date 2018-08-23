extern crate jpeg_encoder;
extern crate jpeg_decoder;

use jpeg_encoder::JpegEncoder;
use jpeg_decoder::Decoder;
use jpeg_encoder::Color;
use std::io::Cursor;

#[test]
fn roundtrip_sanity_check() {
    // create a 1x1 8-bit image buffer containing a single red pixel
    let img = [255u8, 0, 0];

    // encode it into a memory buffer
    let mut encoded_img = Vec::new();
    {
        let mut encoder = JpegEncoder::new_with_quality(&mut encoded_img, 100);
        encoder
            .encode(&img, 1, 1, Color::RGB)
            .expect("Could not encode image");
    }

    // decode it from the memory buffer
    {
        let mut decoder = Decoder::new(Cursor::new(&encoded_img));
        let decoded = decoder.decode().expect("Could not decode image");
        assert_eq!(3, decoded.len());
        assert!(decoded[0] > 0x80);
        assert!(decoded[1] < 0x80);
        assert!(decoded[2] < 0x80);
    }
}

#[test]
fn grayscale_roundtrip_sanity_check() {
    // create a 2x2 8-bit image buffer containing a white diagonal
    let img = [255u8, 0, 0, 255];

    // encode it into a memory buffer
    let mut encoded_img = Vec::new();
    {
        let mut encoder = JpegEncoder::new_with_quality(&mut encoded_img, 100);
        encoder
            .encode(&img, 2, 2, Color::Gray)
            .expect("Could not encode image");
    }

    // decode it from the memory buffer
    {
        let mut decoder = Decoder::new(Cursor::new(&encoded_img));
        let decoded = decoder.decode().expect("Could not decode image");
        assert_eq!(4, decoded.len());
        assert!(decoded[0] > 0x80);
        assert!(decoded[1] < 0x80);
        assert!(decoded[2] < 0x80);
        assert!(decoded[3] > 0x80);
    }
}
