//! Shared image decoding, EXIF orientation normalization, resizing, blurring, and WebP encoding helpers.

use std::io::Cursor;

use image::{
    DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits,
    imageops::{FilterType, blur},
    metadata::Orientation,
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

/// Validate byte length, decode an uploaded image from memory, and apply any EXIF orientation.
pub fn decode_image_within_limit(
    bytes: &[u8],
    max_bytes: usize,
) -> Result<DynamicImage, MediaProcessingError> {
    if bytes.is_empty() || bytes.len() > max_bytes {
        return Err(MediaProcessingError::InvalidSize);
    }

    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| MediaProcessingError::DecodeFailed)?;
    let mut decoder = reader
        .into_decoder()
        .map_err(|_| MediaProcessingError::DecodeFailed)?;

    let mut limits = Limits::default();
    limits
        .reserve(decoder.total_bytes())
        .and_then(|()| decoder.set_limits(limits))
        .map_err(|_| MediaProcessingError::DecodeFailed)?;

    let orientation =
        orientation_from_decoder(&mut decoder).map_err(|_| MediaProcessingError::DecodeFailed)?;

    let mut img =
        DynamicImage::from_decoder(decoder).map_err(|_| MediaProcessingError::DecodeFailed)?;
    img.apply_orientation(orientation);
    Ok(img)
}

fn orientation_from_decoder<D: ImageDecoder>(decoder: &mut D) -> image::ImageResult<Orientation> {
    match decoder.orientation()? {
        Orientation::NoTransforms => Ok(decoder
            .exif_metadata()?
            .as_deref()
            .and_then(|chunk| {
                // Some non-standard PNG/WebP encoders keep the JPEG "Exif\0\0" prefix in the chunk.
                let tiff = chunk.strip_prefix(b"Exif\0\0").unwrap_or(chunk);
                Orientation::from_exif_chunk(tiff)
            })
            .unwrap_or(Orientation::NoTransforms)),
        orientation => Ok(orientation),
    }
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
    use image::{
        ImageEncoder, Rgb, RgbImage,
        codecs::{jpeg::JpegEncoder, png::PngEncoder, webp::WebPEncoder},
    };

    use super::*;

    /// Generate valid PNG bytes for unit tests.
    pub(crate) fn png_bytes_for_test(width: u32, height: u32) -> Vec<u8> {
        let img = DynamicImage::new_rgb8(width, height);
        let mut out = Cursor::new(Vec::new());
        img.write_to(&mut out, ImageFormat::Png).unwrap();
        out.into_inner()
    }

    /// Build a minimal TIFF EXIF payload with an extra tag and the given orientation value.
    pub(crate) fn exif_tiff_chunk(orientation: u16, big_endian: bool) -> Vec<u8> {
        let mut buf = Vec::new();
        let push_u16 = |b: &mut Vec<u8>, v: u16| {
            b.extend_from_slice(&if big_endian {
                v.to_be_bytes()
            } else {
                v.to_le_bytes()
            });
        };
        let push_u32 = |b: &mut Vec<u8>, v: u32| {
            b.extend_from_slice(&if big_endian {
                v.to_be_bytes()
            } else {
                v.to_le_bytes()
            });
        };

        buf.extend_from_slice(if big_endian { b"MM\x00*" } else { b"II*\x00" });
        push_u32(&mut buf, 8);
        push_u16(&mut buf, 2);

        // Tag 0x010F (Make) before Orientation to exercise IFD scanning.
        push_u16(&mut buf, 0x010F);
        push_u16(&mut buf, 2);
        push_u32(&mut buf, 4);
        buf.extend_from_slice(b"Cam\0");

        // Tag 0x0112 (Orientation), SHORT (3), count 1.
        push_u16(&mut buf, 0x0112);
        push_u16(&mut buf, 3);
        push_u32(&mut buf, 1);
        push_u16(&mut buf, orientation);
        push_u16(&mut buf, 0);

        push_u32(&mut buf, 0);
        buf
    }

    /// Build a 64x32 image with 4 distinct quadrant colors: TL=Red, TR=Green, BL=Blue, BR=Yellow.
    pub(crate) fn quadrant_image_for_test(width: u32, height: u32) -> DynamicImage {
        let mut rgb = RgbImage::new(width, height);
        for (x, y, px) in rgb.enumerate_pixels_mut() {
            *px = match (x < width / 2, y < height / 2) {
                (true, true) => Rgb([240, 20, 20]),
                (false, true) => Rgb([20, 240, 20]),
                (true, false) => Rgb([20, 20, 240]),
                (false, false) => Rgb([240, 240, 20]),
            };
        }
        DynamicImage::ImageRgb8(rgb)
    }

    pub(crate) fn encode_with_exif(
        img: &DynamicImage,
        format: ImageFormat,
        exif: Vec<u8>,
    ) -> Vec<u8> {
        let rgb = img.to_rgb8();
        let (w, h) = (rgb.width(), rgb.height());
        let mut out = Cursor::new(Vec::new());
        match format {
            ImageFormat::Jpeg => {
                let mut enc = JpegEncoder::new_with_quality(&mut out, 95);
                enc.set_exif_metadata(exif).unwrap();
                enc.write_image(&rgb, w, h, image::ExtendedColorType::Rgb8)
                    .unwrap();
            }
            ImageFormat::Png => {
                let mut enc = PngEncoder::new(&mut out);
                enc.set_exif_metadata(exif).unwrap();
                enc.write_image(&rgb, w, h, image::ExtendedColorType::Rgb8)
                    .unwrap();
            }
            ImageFormat::WebP => {
                let mut enc = WebPEncoder::new_lossless(&mut out);
                enc.set_exif_metadata(exif).unwrap();
                enc.write_image(&rgb, w, h, image::ExtendedColorType::Rgb8)
                    .unwrap();
            }
            other => panic!("unsupported test format: {other:?}"),
        }
        out.into_inner()
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) enum QuadrantColor {
        Red,
        Green,
        Blue,
        Yellow,
    }

    pub(crate) fn classify_pixel(img: &DynamicImage, x: u32, y: u32) -> QuadrantColor {
        let rgb = img.to_rgb8();
        let [r, g, b] = rgb.get_pixel(x, y).0;
        match (r > 150, g > 150, b > 150) {
            (true, false, false) => QuadrantColor::Red,
            (false, true, false) => QuadrantColor::Green,
            (false, false, true) => QuadrantColor::Blue,
            (true, true, false) => QuadrantColor::Yellow,
            _ => panic!("unexpected color at ({x}, {y}): [{r}, {g}, {b}]"),
        }
    }

    pub(crate) fn assert_quadrants(img: &DynamicImage, expected: [QuadrantColor; 4]) {
        let (w, h) = (img.width(), img.height());
        let actual = [
            classify_pixel(img, w / 4, h / 4),
            classify_pixel(img, 3 * w / 4, h / 4),
            classify_pixel(img, w / 4, 3 * h / 4),
            classify_pixel(img, 3 * w / 4, 3 * h / 4),
        ];
        assert_eq!(actual, expected);
    }

    pub(crate) fn assert_no_exif_metadata(webp_bytes: &[u8]) {
        let mut decoder = ImageReader::new(Cursor::new(webp_bytes))
            .with_guessed_format()
            .unwrap()
            .into_decoder()
            .unwrap();
        assert_eq!(decoder.exif_metadata().unwrap(), None);
        assert_eq!(decoder.orientation().unwrap(), Orientation::NoTransforms);
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

    #[test_log::test]
    fn applies_all_exif_orientations_and_strips_exif_on_webp_encode() {
        use QuadrantColor::{Blue as B, Green as G, Red as R, Yellow as Y};

        let cases = [
            (1, 64, 32, [R, G, B, Y]),
            (2, 64, 32, [G, R, Y, B]),
            (3, 64, 32, [Y, B, G, R]),
            (4, 64, 32, [B, Y, R, G]),
            (5, 32, 64, [R, B, G, Y]),
            (6, 32, 64, [B, R, Y, G]),
            (7, 32, 64, [Y, G, B, R]),
            (8, 32, 64, [G, Y, R, B]),
        ];

        let src = quadrant_image_for_test(64, 32);
        for format in [ImageFormat::Jpeg, ImageFormat::Png, ImageFormat::WebP] {
            for big_endian in [false, true] {
                for (exif_orientation, exp_w, exp_h, exp_quads) in cases {
                    let raw = encode_with_exif(
                        &src,
                        format,
                        exif_tiff_chunk(exif_orientation, big_endian),
                    );
                    let decoded = decode_image_within_limit(&raw, 1024 * 1024).unwrap();
                    assert_eq!(
                        (decoded.width(), decoded.height()),
                        (exp_w, exp_h),
                        "format={format:?} be={big_endian} orientation={exif_orientation}"
                    );
                    assert_quadrants(&decoded, exp_quads);

                    let webp = encode_webp(&decoded).unwrap();
                    assert_no_exif_metadata(&webp);
                    let stored = image::load_from_memory(&webp).unwrap();
                    assert_eq!((stored.width(), stored.height()), (exp_w, exp_h));
                    assert_quadrants(&stored, exp_quads);
                }
            }
        }
    }

    #[test_log::test]
    fn handles_prefixed_exif_chunk_and_invalid_orientation_tag() {
        use QuadrantColor::{Blue as B, Green as G, Red as R, Yellow as Y};

        let src = quadrant_image_for_test(64, 32);

        let mut prefixed = b"Exif\0\0".to_vec();
        prefixed.extend_from_slice(&exif_tiff_chunk(6, false));
        let webp_with_prefix = encode_with_exif(&src, ImageFormat::WebP, prefixed);
        let decoded = decode_image_within_limit(&webp_with_prefix, 1024 * 1024).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (32, 64));
        assert_quadrants(&decoded, [B, R, Y, G]);

        let invalid_orient = encode_with_exif(&src, ImageFormat::Jpeg, exif_tiff_chunk(99, true));
        let decoded_invalid = decode_image_within_limit(&invalid_orient, 1024 * 1024).unwrap();
        assert_eq!(
            (decoded_invalid.width(), decoded_invalid.height()),
            (64, 32)
        );
        assert_quadrants(&decoded_invalid, [R, G, B, Y]);
    }
}
