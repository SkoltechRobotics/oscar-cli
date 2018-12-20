# OS:Car utilities
This crate contains utilities used in the OS:Car project.

## Utilities
### `pnm2flif`
Converts PNM frames in the given directory recorded using `oscar-rec`
application to RGBA FLIF format with an additionall flipping. Resulting images
can be converted to other formats using `convert` application. Additionally it
can be used to verify equality of PNM and RGBA FLIF frames.

### `convert`
Converts RGBA FLIF frames to on the supported formats (PNM, PNG, JPEG). It can
apply demosaicing, histogram equalization and resizing to the images before
saving them.

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.