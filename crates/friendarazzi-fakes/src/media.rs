use std::io::Cursor;

use image::{DynamicImage, ImageFormat};

/// Generate valid PNG bytes for integration and E2E tests.
pub fn png_bytes_for_test(width: u32, height: u32) -> Vec<u8> {
    let img = DynamicImage::new_rgb8(width, height);
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, ImageFormat::Png).unwrap();
    out.into_inner()
}
