use libc;

mod ffi;
mod util;

pub use self::util::read_flif;

#[derive(Debug, Copy, Clone)]
pub struct DecodingError;

pub struct FlifDecoder {
    decoder: *mut ffi::FlifDecoder,
    width: u32,
    height: u32,
    chans: u8,
    depth: u8,
    images: usize,
}

impl Drop for FlifDecoder {
    fn drop(&mut self) {
        unsafe { ffi::flif_destroy_decoder(self.decoder) };
    }
}

impl FlifDecoder {
    pub fn new(data: &[u8]) -> Result<FlifDecoder, DecodingError> {
        unsafe {
            let decoder = ffi::flif_create_decoder();
            let res = ffi::flif_decoder_decode_memory(
                decoder,
                data.as_ptr() as *const libc::c_void,
                data.len(),
            );
            if res == 0 { Err(DecodingError)? }
            let info = ffi::flif_read_info_from_memory(
                data.as_ptr() as *const libc::c_void,
                data.len(),
            );
            if info.is_null() { Err(DecodingError)? }
            let width = ffi::flif_info_get_width(info);
            let height = ffi::flif_info_get_height(info);
            let chans = ffi::flif_info_get_nb_channels(info);
            let depth = ffi::flif_info_get_depth(info);
            let images = ffi::flif_decoder_num_images(decoder);
            ffi::flif_destroy_info(info);
            Ok(FlifDecoder { decoder, width, height, chans, depth, images })
        }
    }


    pub fn width(&self) -> u32 { self.width }

    pub fn height(&self) -> u32 { self.height }

    pub fn channels(&self) -> u8 { self.chans }

    pub fn depth(&self) -> u8 { self.depth }

    pub fn frames(&self) -> u32 { self.images as u32 }

    pub fn get_image_data(&self, n: usize) -> Box<[u8]> {
        let image = unsafe { ffi::flif_decoder_get_image(self.decoder, n) };
        let w = self.width as usize;
        let h = self.height as usize;
        // bytes per pixel
        let bpp = (self.chans as usize)*((self.depth/8) as usize);
        if n >= self.images { panic!("invalid image index") }
        let mut buf = vec![0u8; (w*h*bpp) as usize];
        for i in 0..h {
            unsafe {
                // bytes per row
                let bpr = w*bpp;
                let ptr = buf.as_mut_ptr().offset((i*bpr) as isize)
                    as *mut libc::c_void;
                let i = i as u32;
                match (self.chans, self.depth) {
                    (1, 8) => ffi::flif_image_read_row_GRAY8(image, i, ptr, bpr),
                    _ => panic!("Unsupported number of channels and depth"),
                };
            }
        }
        buf.into_boxed_slice()
    }
}
