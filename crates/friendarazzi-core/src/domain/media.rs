//! Shared image decoding, resizing, blurring, and WebP encoding helpers.

use std::io::Cursor;

use image::{
    DynamicImage, ImageFormat,
    imageops::{FilterType, blur},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MediaProcessingError {
    #[error("das hochgeladene Bild konnte nicht gelesen werden")]
    DecodeFailed,
    #[error("das Bild konnte nicht kodiert werden")]
    EncodeFailed,
    #[error("die Bilddatei ist leer oder zu groß")]
    InvalidSize,
}

/// Validate byte length and decode an uploaded image from memory.
pub fn decode_image_within_limit(
    bytes: &[u8],
    max_bytes: usize,
) -> Result<DynamicImage, MediaProcessingError> {
    if bytes.is_empty() || bytes.len() > max_bytes {
        return Err(MediaProcessingError::InvalidSize);
    }
    image::load_from_memory(bytes).map_err(|_| MediaProcessingError::DecodeFailed)
}

/// Scale down preserving aspect ratio if either dimension exceeds `max_edge`.
pub fn resize_max_edge(img: &DynamicImage, max_edge: u32) -> DynamicImage {
    if img.width() <= max_edge && img.height() <= max_edge {
        return img.clone();
    }
    img.resize(max_edge, max_edge, FilterType::Triangle)
}

/// Crop and scale an image to an exact `edge x edge` square.
pub fn resize_square(img: &DynamicImage, edge: u32) -> DynamicImage {
    img.resize_to_fill(edge, edge, FilterType::Triangle)
}

/// Downsample heavily and apply Gaussian blur so details cannot be deciphered.
pub fn pixelate_and_blur(
    img: &DynamicImage,
    pixel_edge: u32,
    output_edge: u32,
    sigma: f32,
) -> DynamicImage {
    let tiny = img.resize_exact(pixel_edge, pixel_edge, FilterType::Triangle);
    let pixelated = tiny.resize_exact(output_edge, output_edge, FilterType::Nearest);
    let blurred_rgba = blur(&pixelated.to_rgba8(), sigma);
    DynamicImage::ImageRgba8(blurred_rgba)
}

/// Encode an image as WebP.
pub fn encode_webp(img: &DynamicImage) -> Result<Vec<u8>, MediaProcessingError> {
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, ImageFormat::WebP)
        .map_err(|_| MediaProcessingError::EncodeFailed)?;
    Ok(out.into_inner())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Generate valid PNG bytes for unit tests.
    pub(crate) fn png_bytes_for_test(width: u32, height: u32) -> Vec<u8> {
        let img = DynamicImage::new_rgb8(width, height);
        let mut out = Cursor::new(Vec::new());
        img.write_to(&mut out, ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test_log::test]
    fn encodes_and_decodes_webp_and_rejects_invalid_bytes() {
        let raw = png_bytes_for_test(320, 160);
        let decoded = decode_image_within_limit(&raw, 1024 * 1024).unwrap();
        let resized = resize_max_edge(&decoded, 160);
        let webp = encode_webp(&resized).unwrap();
        let roundtrip = image::load_from_memory(&webp).unwrap();
        assert_eq!(roundtrip.width(), 160);
        assert_eq!(roundtrip.height(), 80);

        assert!(matches!(
            decode_image_within_limit(&[], 1024),
            Err(MediaProcessingError::InvalidSize)
        ));
        assert!(matches!(
            decode_image_within_limit(b"not an image", 1024),
            Err(MediaProcessingError::DecodeFailed)
        ));
    }
}
