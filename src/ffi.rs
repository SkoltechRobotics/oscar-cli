use libc::{int32_t, uint8_t, uint32_t, size_t, c_void};

#[link(name = "flif_dec")]
extern "C" {
    pub type FlifDecoder;
    pub type FlifImage;
    pub type FlifInfo;

    pub fn flif_create_decoder() -> *mut FlifDecoder;
    pub fn flif_destroy_decoder(ptr: *mut FlifDecoder);
    pub fn flif_decoder_decode_memory(
            ptr: *mut FlifDecoder, buf: *const c_void, size: size_t,
        ) -> int32_t;
    pub fn flif_decoder_get_image(ptr: *mut FlifDecoder, index: size_t)
        -> *mut FlifImage;
    pub fn flif_decoder_num_images(ptr: *mut FlifDecoder) -> size_t;

    pub fn flif_image_read_row_GRAY8(image: *mut FlifImage, row: uint32_t,
        buf: *mut c_void, size: size_t);

    pub fn flif_read_info_from_memory(buf: *const c_void, size: size_t)
        -> *mut FlifInfo;
    pub fn flif_destroy_info(ptr: *mut FlifInfo);

    pub fn flif_info_get_width(ptr: *mut FlifInfo) -> uint32_t;
    pub fn flif_info_get_height(ptr: *mut FlifInfo) -> uint32_t;
    pub fn flif_info_get_nb_channels(ptr: *mut FlifInfo) -> uint8_t;
    pub fn flif_info_get_depth(ptr: *mut FlifInfo) -> uint8_t;

}