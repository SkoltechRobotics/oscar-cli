//! Decoding and Encoding of JPEG Images
//!
//! JPEG (Joint Photographic Experts Group) is an image format that supports lossy compression.
//! This module implements the Baseline JPEG standard.
//!
//! # Related Links
//! * <http://www.w3.org/Graphics/JPEG/itu-t81.pdf> - The JPEG specification
//!
extern crate byteorder;
extern crate num_iter;

mod entropy;
mod transform;
mod consts;

use byteorder::{BigEndian, WriteBytesExt};
use num_iter::range_step;
use std::io::{self, Write};

use entropy::build_huff_lut;
use consts::*;

pub enum Color {
    RGB,
    RGBA,
    Gray,
    GrayA,
}

/// A representation of a JPEG component
#[derive(Copy, Clone)]
struct Component {
    /// The Component's identifier
    id: u8,
    /// Horizontal sampling factor
    h: u8,
    /// Vertical sampling factor
    v: u8,
    /// The quantization table selector
    tq: u8,
    /// Index to the Huffman DC Table
    dc_table: u8,
    /// Index to the AC Huffman Table
    ac_table: u8,
    /// The dc prediction of the component
    _dc_pred: i32,
}

pub struct BitWriter<'a, W: 'a> {
    w: &'a mut W,
    accumulator: u32,
    nbits: u8,
}

impl<'a, W: Write + 'a> BitWriter<'a, W> {
    fn new(w: &'a mut W) -> Self {
        BitWriter {
            w,
            accumulator: 0,
            nbits: 0,
        }
    }

    fn write_bits(&mut self, bits: u16, size: u8) -> io::Result<()> {
        if size == 0 {
            return Ok(());
        }

        self.accumulator |= u32::from(bits) << (32 - (self.nbits + size)) as usize;
        self.nbits += size;

        while self.nbits >= 8 {
            let byte = (self.accumulator & (0xFFFF_FFFFu32 << 24)) >> 24;
            try!(self.w.write_all(&[byte as u8]));

            if byte == 0xFF {
                try!(self.w.write_all(&[0x00]));
            }

            self.nbits -= 8;
            self.accumulator <<= 8;
        }

        Ok(())
    }

    fn pad_byte(&mut self) -> io::Result<()> {
        self.write_bits(0x7F, 7)
    }

    fn huffman_encode(&mut self, val: u8, table: &[(u8, u16)]) -> io::Result<()> {
        let (size, code) = table[val as usize];

        if size > 16 {
            panic!("bad huffman value");
        }

        self.write_bits(code, size)
    }

    fn write_block(
        &mut self,
        block: &[i32],
        prevdc: i32,
        dctable: &[(u8, u16)],
        actable: &[(u8, u16)],
    ) -> io::Result<i32> {
        // Differential DC encoding
        let dcval = block[0];
        let diff = dcval - prevdc;
        let (size, value) = encode_coefficient(diff);

        try!(self.huffman_encode(size, dctable));
        try!(self.write_bits(value, size));

        // Figure F.2
        let mut zero_run = 0;
        let mut k = 0usize;

        loop {
            k += 1;

            if block[UNZIGZAG[k] as usize] == 0 {
                if k == 63 {
                    try!(self.huffman_encode(0x00, actable));
                    break;
                }

                zero_run += 1;
            } else {
                while zero_run > 15 {
                    try!(self.huffman_encode(0xF0, actable));
                    zero_run -= 16;
                }

                let (size, value) = encode_coefficient(block[UNZIGZAG[k] as usize]);
                let symbol = (zero_run << 4) | size;

                try!(self.huffman_encode(symbol, actable));
                try!(self.write_bits(value, size));

                zero_run = 0;

                if k == 63 {
                    break;
                }
            }
        }

        Ok(dcval)
    }

    fn write_segment(&mut self, marker: u8, data: Option<&[u8]>) -> io::Result<()> {
        try!(self.w.write_all(&[0xFF]));
        try!(self.w.write_all(&[marker]));

        if let Some(b) = data {
            try!(self.w.write_u16::<BigEndian>(b.len() as u16 + 2));
            try!(self.w.write_all(b));
        }
        Ok(())
    }
}

/// The representation of a JPEG encoder
pub struct JpegEncoder<'a, W: 'a> {
    writer: BitWriter<'a, W>,

    components: Vec<Component>,
    tables: Vec<u8>,

    luma_dctable: Vec<(u8, u16)>,
    luma_actable: Vec<(u8, u16)>,
    chroma_dctable: Vec<(u8, u16)>,
    chroma_actable: Vec<(u8, u16)>,
}

impl<'a, W: Write> JpegEncoder<'a, W> {
    /// Create a new encoder that writes its output to ```w```
    pub fn new(w: &mut W) -> JpegEncoder<W> {
        JpegEncoder::new_with_quality(w, 75)
    }

    /// Create a new encoder that writes its output to ```w```, and has
    /// the quality parameter ```quality``` with a value in the range 1-100
    /// where 1 is the worst and 100 is the best.
    pub fn new_with_quality(w: &mut W, quality: u8) -> JpegEncoder<W> {
        let ld = build_huff_lut(&STD_LUMA_DC_CODE_LENGTHS, &STD_LUMA_DC_VALUES);
        let la = build_huff_lut(&STD_LUMA_AC_CODE_LENGTHS, &STD_LUMA_AC_VALUES);

        let cd = build_huff_lut(&STD_CHROMA_DC_CODE_LENGTHS, &STD_CHROMA_DC_VALUES);
        let ca = build_huff_lut(&STD_CHROMA_AC_CODE_LENGTHS, &STD_CHROMA_AC_VALUES);

        let components = vec![
            Component {
                id: LUMAID,
                h: 1,
                v: 1,
                tq: LUMADESTINATION,
                dc_table: LUMADESTINATION,
                ac_table: LUMADESTINATION,
                _dc_pred: 0,
            },
            Component {
                id: CHROMABLUEID,
                h: 1,
                v: 1,
                tq: CHROMADESTINATION,
                dc_table: CHROMADESTINATION,
                ac_table: CHROMADESTINATION,
                _dc_pred: 0,
            },
            Component {
                id: CHROMAREDID,
                h: 1,
                v: 1,
                tq: CHROMADESTINATION,
                dc_table: CHROMADESTINATION,
                ac_table: CHROMADESTINATION,
                _dc_pred: 0,
            },
        ];

        // Derive our quantization table scaling value using the libjpeg algorithm
        let scale = u32::from(clamp(quality, 1, 100));
        let scale = if scale < 50 {
            5000 / scale
        } else {
            200 - scale * 2
        };

        let mut tables = Vec::new();
        let scale_value = |&v: &u8| {
            let value = (u32::from(v) * scale + 50) / 100;

            clamp(value, 1, u32::from(u8::max_value())) as u8
        };
        tables.extend(STD_LUMA_QTABLE.iter().map(&scale_value));
        tables.extend(STD_CHROMA_QTABLE.iter().map(&scale_value));

        JpegEncoder {
            writer: BitWriter::new(w),

            components,
            tables,

            luma_dctable: ld,
            luma_actable: la,
            chroma_dctable: cd,
            chroma_actable: ca,
        }
    }

    /// Encodes the image `image` that has dimensions `width` and `height`
    /// and color ```c```
    ///
    /// The Image in encoded with subsampling ratio 4:2:2
    pub fn encode(
        &mut self, image: &[u8], width: u32, height: u32, c: Color
    ) -> io::Result<()> {
        let num_components = match c {
            Color::RGB | Color::RGBA => 3,
            Color::Gray | Color::GrayA => 1,
        };

        try!(self.writer.write_segment(SOI, None));

        let mut buf = Vec::new();

        build_jfif_header(&mut buf);
        try!(self.writer.write_segment(APP0, Some(&buf)));

        build_frame_header(
            &mut buf,
            8,
            width as u16,
            height as u16,
            &self.components[..num_components],
        );
        try!(self.writer.write_segment(SOF0, Some(&buf)));

        assert_eq!(self.tables.len() / 64, 2);
        let numtables = if num_components == 1 { 1 } else { 2 };

        for (i, table) in self.tables.chunks(64).enumerate().take(numtables) {
            build_quantization_segment(&mut buf, 8, i as u8, table);
            try!(self.writer.write_segment(DQT, Some(&buf)));
        }

        build_huffman_segment(
            &mut buf,
            DCCLASS,
            LUMADESTINATION,
            &STD_LUMA_DC_CODE_LENGTHS,
            &STD_LUMA_DC_VALUES,
        );
        try!(self.writer.write_segment(DHT, Some(&buf)));

        build_huffman_segment(
            &mut buf,
            ACCLASS,
            LUMADESTINATION,
            &STD_LUMA_AC_CODE_LENGTHS,
            &STD_LUMA_AC_VALUES,
        );
        try!(self.writer.write_segment(DHT, Some(&buf)));

        if num_components == 3 {
            build_huffman_segment(
                &mut buf,
                DCCLASS,
                CHROMADESTINATION,
                &STD_CHROMA_DC_CODE_LENGTHS,
                &STD_CHROMA_DC_VALUES,
            );
            try!(self.writer.write_segment(DHT, Some(&buf)));

            build_huffman_segment(
                &mut buf,
                ACCLASS,
                CHROMADESTINATION,
                &STD_CHROMA_AC_CODE_LENGTHS,
                &STD_CHROMA_AC_VALUES,
            );
            try!(self.writer.write_segment(DHT, Some(&buf)));
        }

        build_scan_header(&mut buf, &self.components[..num_components]);
        try!(self.writer.write_segment(SOS, Some(&buf)));

        match c {
            Color::RGB => {
                try!(self.encode_rgb(image, width as usize, height as usize, 3))
            }
            Color::RGBA => {
                try!(self.encode_rgb(image, width as usize, height as usize, 4))
            }
            Color::Gray => {
                try!(self.encode_gray(image, width as usize, height as usize, 1))
            }
            Color::GrayA => {
                try!(self.encode_gray(image, width as usize, height as usize, 2))
            },
        };

        try!(self.writer.pad_byte());
        try!(self.writer.write_segment(EOI, None));
        Ok(())
    }

    fn encode_gray(
        &mut self,
        image: &[u8],
        width: usize,
        height: usize,
        bpp: usize,
    ) -> io::Result<()> {
        let mut yblock = [0u8; 64];
        let mut y_dcprev = 0;
        let mut dct_yblock = [0i32; 64];

        for y in range_step(0, height, 8) {
            for x in range_step(0, width, 8) {
                // RGB -> YCbCr
                copy_blocks_gray(image, x, y, width, bpp, &mut yblock);

                // Level shift and fdct
                // Coeffs are scaled by 8
                transform::fdct(&yblock, &mut dct_yblock);

                // Quantization
                for (i, dct) in dct_yblock.iter_mut().enumerate().take(64) {
                    *dct = ((*dct / 8) as f32 / f32::from(self.tables[i])).round() as i32;
                }

                let la = &*self.luma_actable;
                let ld = &*self.luma_dctable;

                y_dcprev = try!(self.writer.write_block(&dct_yblock, y_dcprev, ld, la));
            }
        }

        Ok(())
    }

    fn encode_rgb(
        &mut self,
        image: &[u8],
        width: usize,
        height: usize,
        bpp: usize,
    ) -> io::Result<()> {
        let mut y_dcprev = 0;
        let mut cb_dcprev = 0;
        let mut cr_dcprev = 0;

        let mut dct_yblock = [0i32; 64];
        let mut dct_cb_block = [0i32; 64];
        let mut dct_cr_block = [0i32; 64];

        let mut yblock = [0u8; 64];
        let mut cb_block = [0u8; 64];
        let mut cr_block = [0u8; 64];

        for y in range_step(0, height, 8) {
            for x in range_step(0, width, 8) {
                // RGB -> YCbCr
                copy_blocks_ycbcr(
                    image,
                    x,
                    y,
                    width,
                    bpp,
                    &mut yblock,
                    &mut cb_block,
                    &mut cr_block,
                );

                // Level shift and fdct
                // Coeffs are scaled by 8
                transform::fdct(&yblock, &mut dct_yblock);
                transform::fdct(&cb_block, &mut dct_cb_block);
                transform::fdct(&cr_block, &mut dct_cr_block);

                // Quantization
                for i in 0usize..64 {
                    dct_yblock[i] =
                        ((dct_yblock[i] / 8) as f32 / f32::from(self.tables[i])).round() as i32;
                    dct_cb_block[i] = ((dct_cb_block[i] / 8) as f32
                        / f32::from(self.tables[64..][i]))
                        .round() as i32;
                    dct_cr_block[i] = ((dct_cr_block[i] / 8) as f32
                        / f32::from(self.tables[64..][i]))
                        .round() as i32;
                }

                let la = &*self.luma_actable;
                let ld = &*self.luma_dctable;
                let cd = &*self.chroma_dctable;
                let ca = &*self.chroma_actable;

                y_dcprev = try!(self.writer.write_block(&dct_yblock, y_dcprev, ld, la));
                cb_dcprev = try!(self.writer.write_block(&dct_cb_block, cb_dcprev, cd, ca));
                cr_dcprev = try!(self.writer.write_block(&dct_cr_block, cr_dcprev, cd, ca));
            }
        }

        Ok(())
    }
}

fn build_jfif_header(m: &mut Vec<u8>) {
    m.clear();

    let _ = write!(m, "JFIF");
    let _ = m.write_all(&[0]);
    let _ = m.write_all(&[0x01]);
    let _ = m.write_all(&[0x02]);
    let _ = m.write_all(&[0]);
    let _ = m.write_u16::<BigEndian>(1);
    let _ = m.write_u16::<BigEndian>(1);
    let _ = m.write_all(&[0]);
    let _ = m.write_all(&[0]);
}

fn build_frame_header(
    m: &mut Vec<u8>,
    precision: u8,
    width: u16,
    height: u16,
    components: &[Component],
) {
    m.clear();

    let _ = m.write_all(&[precision]);
    let _ = m.write_u16::<BigEndian>(height);
    let _ = m.write_u16::<BigEndian>(width);
    let _ = m.write_all(&[components.len() as u8]);

    for &comp in components.iter() {
        let _ = m.write_all(&[comp.id]);
        let hv = (comp.h << 4) | comp.v;
        let _ = m.write_all(&[hv]);
        let _ = m.write_all(&[comp.tq]);
    }
}

fn build_scan_header(m: &mut Vec<u8>, components: &[Component]) {
    m.clear();

    let _ = m.write_all(&[components.len() as u8]);

    for &comp in components.iter() {
        let _ = m.write_all(&[comp.id]);
        let tables = (comp.dc_table << 4) | comp.ac_table;
        let _ = m.write_all(&[tables]);
    }

    // spectral start and end, approx. high and low
    let _ = m.write_all(&[0]);
    let _ = m.write_all(&[63]);
    let _ = m.write_all(&[0]);
}

fn build_huffman_segment(
    m: &mut Vec<u8>,
    class: u8,
    destination: u8,
    numcodes: &[u8],
    values: &[u8],
) {
    m.clear();

    let tcth = (class << 4) | destination;
    let _ = m.write_all(&[tcth]);

    assert_eq!(numcodes.len(), 16);

    let mut sum = 0usize;

    for &i in numcodes.iter() {
        let _ = m.write_all(&[i]);
        sum += i as usize;
    }

    assert_eq!(sum, values.len());

    for &i in values.iter() {
        let _ = m.write_all(&[i]);
    }
}

fn build_quantization_segment(m: &mut Vec<u8>, precision: u8, identifier: u8, qtable: &[u8]) {
    assert_eq!(qtable.len() % 64, 0);
    m.clear();

    let p = if precision == 8 { 0 } else { 1 };

    let pqtq = (p << 4) | identifier;
    let _ = m.write_all(&[pqtq]);

    for i in 0usize..64 {
        let _ = m.write_all(&[qtable[UNZIGZAG[i] as usize]]);
    }
}

fn encode_coefficient(coefficient: i32) -> (u8, u16) {
    let mut magnitude = coefficient.abs() as u16;
    let mut num_bits = 0u8;

    while magnitude > 0 {
        magnitude >>= 1;
        num_bits += 1;
    }

    let mask = (1 << num_bits as usize) - 1;

    let val = if coefficient < 0 {
        (coefficient - 1) as u16 & mask
    } else {
        coefficient as u16 & mask
    };

    (num_bits, val)
}

fn rgb_to_ycbcr(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
    let r = f32::from(r);
    let g = f32::from(g);
    let b = f32::from(b);

    let y = 0.299f32 * r + 0.587f32 * g + 0.114f32 * b;
    let cb = -0.1687f32 * r - 0.3313f32 * g + 0.5f32 * b + 128f32;
    let cr = 0.5f32 * r - 0.4187f32 * g - 0.0813f32 * b + 128f32;

    (y as u8, cb as u8, cr as u8)
}

fn value_at(s: &[u8], index: usize) -> u8 {
    if index < s.len() {
        s[index]
    } else {
        s[s.len() - 1]
    }
}

fn copy_blocks_ycbcr(
    source: &[u8],
    x0: usize,
    y0: usize,
    width: usize,
    bpp: usize,
    yb: &mut [u8; 64],
    cbb: &mut [u8; 64],
    crb: &mut [u8; 64],
) {
    for y in 0usize..8 {
        let ystride = (y0 + y) * bpp * width;

        for x in 0usize..8 {
            let xstride = x0 * bpp + x * bpp;

            let r = value_at(source, ystride + xstride);
            let g = value_at(source, ystride + xstride + 1);
            let b = value_at(source, ystride + xstride + 2);

            let (yc, cb, cr) = rgb_to_ycbcr(r, g, b);

            yb[y * 8 + x] = yc;
            cbb[y * 8 + x] = cb;
            crb[y * 8 + x] = cr;
        }
    }
}

fn copy_blocks_gray(
    source: &[u8],
    x0: usize,
    y0: usize,
    width: usize,
    bpp: usize,
    gb: &mut [u8; 64],
) {
    for y in 0usize..8 {
        let ystride = (y0 + y) * bpp * width;

        for x in 0usize..8 {
            let xstride = x0 * bpp + x * bpp;
            gb[y * 8 + x] = value_at(source, ystride + xstride);
        }
    }
}

/// Cut value to be inside given range
fn clamp<N: PartialOrd>(a: N, min: N, max: N) -> N {
    if a < min {
        min
    } else if a > max {
        max
    } else {
        a
    }
}
