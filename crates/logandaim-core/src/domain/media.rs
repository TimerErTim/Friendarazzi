//! Image normalization and blurred preview generation for uploaded photos.

use std::io::Cursor;

use image::{
    DynamicImage, ImageFormat,
    codecs::jpeg::JpegEncoder,
    imageops::{FilterType, blur},
};
use thiserror::Error;

const MAX_CAPTURE_EDGE: u32 = 1600;
const BLUR_SMALL_EDGE: u32 = 14;
const BLUR_OUTPUT_EDGE: u32 = 64;
const AVATAR_EDGE: u32 = 400;

#[derive(Debug, Error)]
pub enum MediaProcessingError {
    #[error("das hochgeladene Bild konnte nicht gelesen werden")]
    DecodeFailed,
    #[error("das Bild konnte nicht kodiert werden")]
    EncodeFailed,
    #[error("die Bilddatei ist leer oder zu groß")]
    InvalidSize,
}

pub struct ProcessedCaptureMedia {
    pub full_jpeg: Vec<u8>,
    pub blurred_jpeg: Vec<u8>,
}

/// Normalize a capture photo to max 1600px JPEG and generate a 64px pixelated+blurred teaser.
pub fn process_capture_photo(bytes: &[u8]) -> Result<ProcessedCaptureMedia, MediaProcessingError> {
    if bytes.is_empty() || bytes.len() > 20 * 1024 * 1024 {
        return Err(MediaProcessingError::InvalidSize);
    }

    let img = image::load_from_memory(bytes).map_err(|_| MediaProcessingError::DecodeFailed)?;
    let full = resize_max_edge(&img, MAX_CAPTURE_EDGE);
    let full_jpeg = encode_jpeg(&full, 82)?;

    // Pixelate heavily then apply Gaussian blur so targets/prompts cannot be deciphered.
    let tiny = img.resize_exact(BLUR_SMALL_EDGE, BLUR_SMALL_EDGE, FilterType::Triangle);
    let pixelated = tiny.resize_exact(BLUR_OUTPUT_EDGE, BLUR_OUTPUT_EDGE, FilterType::Nearest);
    let blurred_rgba = blur(&pixelated.to_rgba8(), 2.2);
    let blurred = DynamicImage::ImageRgba8(blurred_rgba);
    let blurred_jpeg = encode_jpeg(&blurred, 70)?;

    Ok(ProcessedCaptureMedia {
        full_jpeg,
        blurred_jpeg,
    })
}

/// Normalize a user avatar image to a square 400x400 JPEG.
pub fn process_avatar_photo(bytes: &[u8]) -> Result<Vec<u8>, MediaProcessingError> {
    if bytes.is_empty() || bytes.len() > 10 * 1024 * 1024 {
        return Err(MediaProcessingError::InvalidSize);
    }
    let img = image::load_from_memory(bytes).map_err(|_| MediaProcessingError::DecodeFailed)?;
    let square = img.resize_to_fill(AVATAR_EDGE, AVATAR_EDGE, FilterType::Triangle);
    encode_jpeg(&square, 84)
}

fn resize_max_edge(img: &DynamicImage, max_edge: u32) -> DynamicImage {
    let w = img.width();
    let h = img.height();
    if w <= max_edge && h <= max_edge {
        return img.clone();
    }
    img.resize(max_edge, max_edge, FilterType::Triangle)
}

fn encode_jpeg(img: &DynamicImage, quality: u8) -> Result<Vec<u8>, MediaProcessingError> {
    let rgb = img.to_rgb8();
    let mut out = Cursor::new(Vec::new());
    let encoder = JpegEncoder::new_with_quality(&mut out, quality);
    rgb.write_with_encoder(encoder)
        .map_err(|_| MediaProcessingError::EncodeFailed)?;
    Ok(out.into_inner())
}

/// Detect image media format extension if needed (unused helper kept private to avoid warnings).
pub fn png_bytes_for_test(width: u32, height: u32) -> Vec<u8> {
    let img = DynamicImage::new_rgb8(width, height);
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, ImageFormat::Png).unwrap();
    out.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_log::test]
    fn processes_large_image_into_bounded_jpeg_and_64px_blur() {
        let raw = png_bytes_for_test(2400, 1200);
        let processed = process_capture_photo(&raw).unwrap();

        let full = image::load_from_memory(&processed.full_jpeg).unwrap();
        assert_eq!(full.width(), 1600);
        assert_eq!(full.height(), 800);

        let teaser = image::load_from_memory(&processed.blurred_jpeg).unwrap();
        assert_eq!(teaser.width(), 64);
        assert_eq!(teaser.height(), 64);
    }

    #[test_log::test]
    fn rejects_empty_or_corrupt_bytes() {
        assert!(process_capture_photo(&[]).is_err());
        assert!(process_capture_photo(b"not an image").is_err());
    }
}
