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

    let orientation = orientation_from_decoder_and_bytes(&mut decoder, bytes)
        .map_err(|_| MediaProcessingError::DecodeFailed)?;

    let mut img =
        DynamicImage::from_decoder(decoder).map_err(|_| MediaProcessingError::DecodeFailed)?;
    img.apply_orientation(orientation);
    Ok(img)
}

fn orientation_from_decoder_and_bytes<D: ImageDecoder>(
    decoder: &mut D,
    bytes: &[u8],
) -> image::ImageResult<Orientation> {
    let decoded = decoder.orientation()?;
    if decoded != Orientation::NoTransforms {
        return Ok(decoded);
    }

    // zune-jpeg overwrites exif_data on multiple APP1 segments; scan JPEG APP1 headers directly.
    if bytes.starts_with(&[0xFF, 0xD8])
        && let Some(orientation) = scan_jpeg_app1_orientation(bytes)
    {
        return Ok(orientation);
    }

    if let Some(exif) = decoder.exif_metadata()?
        && let Some(orientation) = parse_exif_orientation(&exif)
    {
        return Ok(orientation);
    }

    if let Some(xmp) = decoder.xmp_metadata()?
        && let Some(orientation) = parse_xmp_orientation(&xmp)
    {
        return Ok(orientation);
    }

    Ok(Orientation::NoTransforms)
}

#[derive(Clone, Copy)]
enum Endian {
    Little,
    Big,
}

impl Endian {
    fn read_u16(self, bytes: &[u8]) -> Option<u16> {
        let arr: [u8; 2] = bytes.get(..2)?.try_into().ok()?;
        Some(match self {
            Self::Little => u16::from_le_bytes(arr),
            Self::Big => u16::from_be_bytes(arr),
        })
    }

    fn read_u32(self, bytes: &[u8]) -> Option<u32> {
        let arr: [u8; 4] = bytes.get(..4)?.try_into().ok()?;
        Some(match self {
            Self::Little => u32::from_le_bytes(arr),
            Self::Big => u32::from_be_bytes(arr),
        })
    }
}

fn parse_exif_orientation(chunk: &[u8]) -> Option<Orientation> {
    parse_tiff_orientation(chunk.strip_prefix(b"Exif\0\0").unwrap_or(chunk))
}

fn parse_tiff_orientation(tiff: &[u8]) -> Option<Orientation> {
    let endian = match tiff.get(..4)? {
        [0x49, 0x49, 42, 0] => Endian::Little,
        [0x4D, 0x4D, 0, 42] => Endian::Big,
        _ => return None,
    };
    let ifd0_offset = usize::try_from(endian.read_u32(tiff.get(4..8)?)?).ok()?;
    let (orientation, exif_ifd_offset) = scan_ifd_for_orientation(tiff, ifd0_offset, endian);
    if orientation.is_some() {
        return orientation;
    }
    let sub_offset = exif_ifd_offset.filter(|&off| off != ifd0_offset)?;
    scan_ifd_for_orientation(tiff, sub_offset, endian).0
}

fn scan_ifd_for_orientation(
    tiff: &[u8],
    offset: usize,
    endian: Endian,
) -> (Option<Orientation>, Option<usize>) {
    let Some(count_bytes) = tiff.get(offset..offset.saturating_add(2)) else {
        return (None, None);
    };
    let Some(entries) = endian.read_u16(count_bytes) else {
        return (None, None);
    };
    let mut exif_ifd_offset = None;
    for idx in 0..usize::from(entries) {
        let entry_start = offset + 2 + idx * 12;
        let Some(entry) = tiff.get(entry_start..entry_start + 12) else {
            break;
        };
        let Some(tag) = endian.read_u16(&entry[0..2]) else {
            break;
        };
        let Some(format) = endian.read_u16(&entry[2..4]) else {
            break;
        };
        let Some(count) = endian.read_u32(&entry[4..8]) else {
            break;
        };
        let value_field = &entry[8..12];

        if tag == 0x0112 {
            let value = if count == 1 {
                match format {
                    1 | 6 => Some(u32::from(value_field[0])),
                    3 | 8 => endian.read_u16(value_field).map(u32::from),
                    4 | 9 => endian.read_u32(value_field),
                    _ => None,
                }
            } else {
                None
            };
            let ascii_value = (format == 2 && (1..=4).contains(&count))
                .then(|| {
                    std::str::from_utf8(&value_field[..count as usize])
                        .ok()
                        .and_then(|s| s.trim_matches(['\0', ' ']).parse::<u8>().ok())
                })
                .flatten();
            if let Some(orient) = ascii_value
                .or_else(|| value.and_then(|v| u8::try_from(v).ok()))
                .and_then(Orientation::from_exif)
            {
                return (Some(orient), exif_ifd_offset);
            }
        } else if tag == 0x8769 && (format == 4 || format == 13) && count == 1 {
            exif_ifd_offset = endian
                .read_u32(value_field)
                .and_then(|v| usize::try_from(v).ok());
        }
    }
    (None, exif_ifd_offset)
}

fn scan_jpeg_app1_orientation(bytes: &[u8]) -> Option<Orientation> {
    let mut pos = 2;
    let mut saw_exif_identity = false;
    let mut xmp_fallback = None;

    while pos < bytes.len() {
        if bytes[pos] != 0xFF {
            break;
        }
        while pos < bytes.len() && bytes[pos] == 0xFF {
            pos += 1;
        }
        if pos >= bytes.len() {
            break;
        }
        let marker = bytes[pos];
        pos += 1;

        if marker == 0x00 || marker == 0x01 || marker == 0xD8 || (0xD0..=0xD7).contains(&marker) {
            continue;
        }
        if marker == 0xDA || marker == 0xD9 {
            break;
        }
        let Some(len_bytes) = bytes.get(pos..pos + 2) else {
            break;
        };
        let segment_len = usize::from(u16::from_be_bytes([len_bytes[0], len_bytes[1]]));
        if segment_len < 2 || pos + segment_len > bytes.len() {
            break;
        }
        let payload = &bytes[pos + 2..pos + segment_len];

        if marker == 0xE1 {
            if let Some(tiff) = payload.strip_prefix(b"Exif\0\0") {
                match parse_exif_orientation(tiff) {
                    Some(Orientation::NoTransforms) => saw_exif_identity = true,
                    Some(orient) => return Some(orient),
                    None => {}
                }
            } else if xmp_fallback.is_none()
                && let Some(xmp) = payload.strip_prefix(b"http://ns.adobe.com/xap/1.0/\0")
            {
                xmp_fallback = parse_xmp_orientation(xmp);
            }
        }

        pos += segment_len;
    }

    if saw_exif_identity {
        Some(Orientation::NoTransforms)
    } else {
        xmp_fallback
    }
}

fn parse_xmp_orientation(xmp: &[u8]) -> Option<Orientation> {
    let text = String::from_utf8_lossy(xmp);
    for attr_prefix in ["tiff:Orientation=\"", "tiff:Orientation='"] {
        if let Some(idx) = text.find(attr_prefix) {
            let rest = &text[idx + attr_prefix.len()..];
            let quote = attr_prefix.as_bytes()[attr_prefix.len() - 1] as char;
            if let Some(end) = rest.find(quote)
                && let Ok(val) = rest[..end].trim().parse::<u8>()
                && let Some(orient) = Orientation::from_exif(val)
            {
                return Some(orient);
            }
        }
    }

    if let Some(start) = text.find("<tiff:Orientation") {
        let after_tag = &text[start + "<tiff:Orientation".len()..];
        if let Some(gt) = after_tag.find('>') {
            let content = &after_tag[gt + 1..];
            if let Some(close) = content.find("</tiff:Orientation>")
                && let Ok(val) = content[..close].trim().parse::<u8>()
            {
                return Orientation::from_exif(val);
            }
        }
    }

    None
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

    fn inject_jpeg_app1_segments(jpeg: &[u8], segments: &[Vec<u8>]) -> Vec<u8> {
        assert_eq!(&jpeg[..2], &[0xFF, 0xD8]);
        let mut out =
            Vec::with_capacity(jpeg.len() + segments.iter().map(|s| s.len() + 4).sum::<usize>());
        out.extend_from_slice(&jpeg[..2]);
        for payload in segments {
            let len = u16::try_from(payload.len() + 2).unwrap();
            out.extend_from_slice(&[0xFF, 0xE1]);
            out.extend_from_slice(&len.to_be_bytes());
            out.extend_from_slice(payload);
        }
        out.extend_from_slice(&jpeg[2..]);
        out
    }

    fn tiff_without_orientation(big_endian: bool) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(if big_endian { b"MM\x00*" } else { b"II*\x00" });
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
        push_u32(&mut buf, 8);
        push_u16(&mut buf, 1);
        push_u16(&mut buf, 0x010F);
        push_u16(&mut buf, 2);
        push_u32(&mut buf, 4);
        buf.extend_from_slice(b"Cam\0");
        push_u32(&mut buf, 0);
        buf
    }

    fn tiff_with_orientation_format(orientation: u8, format: u16, big_endian: bool) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(if big_endian { b"MM\x00*" } else { b"II*\x00" });
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
        push_u32(&mut buf, 8);
        push_u16(&mut buf, 1);
        push_u16(&mut buf, 0x0112);
        push_u16(&mut buf, format);
        push_u32(&mut buf, 1);
        match format {
            1 => buf.extend_from_slice(&[orientation, 0, 0, 0]),
            4 => push_u32(&mut buf, u32::from(orientation)),
            _ => panic!("unsupported format {format}"),
        }
        push_u32(&mut buf, 0);
        buf
    }

    fn tiff_with_exif_ifd_orientation(orientation: u16, big_endian: bool) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(if big_endian { b"MM\x00*" } else { b"II*\x00" });
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
        push_u32(&mut buf, 8);
        // IFD0 at 8: 1 entry (12B) + next_ifd (4B) => ExifIFD starts at 26.
        push_u16(&mut buf, 1);
        push_u16(&mut buf, 0x8769);
        push_u16(&mut buf, 4);
        push_u32(&mut buf, 1);
        push_u32(&mut buf, 26);
        push_u32(&mut buf, 0);

        push_u16(&mut buf, 1);
        push_u16(&mut buf, 0x0112);
        push_u16(&mut buf, 3);
        push_u32(&mut buf, 1);
        push_u16(&mut buf, orientation);
        push_u16(&mut buf, 0);
        push_u32(&mut buf, 0);
        buf
    }

    #[test_log::test]
    fn handles_multiple_app1_exif_non_short_formats_exif_ifd_and_xmp_orientation() {
        use QuadrantColor::{Blue as B, Green as G, Red as R, Yellow as Y};

        let src = quadrant_image_for_test(64, 32);
        let mut plain_jpeg = Cursor::new(Vec::new());
        src.write_to(&mut plain_jpeg, ImageFormat::Jpeg).unwrap();
        let plain_jpeg = plain_jpeg.into_inner();

        // Regression: secondary APP1 Exif without Orientation must not clobber primary Orientation=6.
        let mut app1_primary = b"Exif\0\0".to_vec();
        app1_primary.extend_from_slice(&exif_tiff_chunk(6, false));
        let mut app1_secondary = b"Exif\0\0".to_vec();
        app1_secondary.extend_from_slice(&tiff_without_orientation(false));
        let multi_app1 = inject_jpeg_app1_segments(&plain_jpeg, &[app1_primary, app1_secondary]);
        let decoded = decode_image_within_limit(&multi_app1, 1024 * 1024).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (32, 64));
        assert_quadrants(&decoded, [B, R, Y, G]);

        // Non-SHORT TIFF formats (BYTE=1, LONG=4) in both endiannesses.
        for format in [1u16, 4u16] {
            for big_endian in [false, true] {
                let raw = encode_with_exif(
                    &src,
                    ImageFormat::Jpeg,
                    tiff_with_orientation_format(6, format, big_endian),
                );
                let decoded = decode_image_within_limit(&raw, 1024 * 1024).unwrap();
                assert_eq!((decoded.width(), decoded.height()), (32, 64));
                assert_quadrants(&decoded, [B, R, Y, G]);
            }
        }

        // ExifIFD fallback when IFD0 omits 0x0112.
        let exif_ifd = encode_with_exif(
            &src,
            ImageFormat::Jpeg,
            tiff_with_exif_ifd_orientation(8, true),
        );
        let decoded = decode_image_within_limit(&exif_ifd, 1024 * 1024).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (32, 64));
        assert_quadrants(&decoded, [G, Y, R, B]);

        // XMP tiff:Orientation fallback (element and attribute forms) when EXIF omits 0x0112.
        let mut xmp_elem = b"http://ns.adobe.com/xap/1.0/\0".to_vec();
        xmp_elem.extend_from_slice(br#"<x:xmpmeta><rdf:Description><tiff:Orientation>6</tiff:Orientation></rdf:Description></x:xmpmeta>"#);
        let jpeg_xmp = inject_jpeg_app1_segments(&plain_jpeg, &[xmp_elem]);
        let decoded = decode_image_within_limit(&jpeg_xmp, 1024 * 1024).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (32, 64));
        assert_quadrants(&decoded, [B, R, Y, G]);

        let mut xmp_attr = b"http://ns.adobe.com/xap/1.0/\0".to_vec();
        xmp_attr.extend_from_slice(br#"<rdf:Description tiff:Orientation="8"/>"#);
        let jpeg_xmp_attr = inject_jpeg_app1_segments(&plain_jpeg, &[xmp_attr.clone()]);
        let decoded = decode_image_within_limit(&jpeg_xmp_attr, 1024 * 1024).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (32, 64));
        assert_quadrants(&decoded, [G, Y, R, B]);

        // Explicit EXIF IFD0 Orientation=1 must take precedence over XMP.
        let mut app1_orient1 = b"Exif\0\0".to_vec();
        app1_orient1.extend_from_slice(&exif_tiff_chunk(1, false));
        let jpeg_exif1_plus_xmp =
            inject_jpeg_app1_segments(&plain_jpeg, &[app1_orient1.clone(), xmp_attr]);
        let decoded = decode_image_within_limit(&jpeg_exif1_plus_xmp, 1024 * 1024).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (64, 32));
        assert_quadrants(&decoded, [R, G, B, Y]);

        // Secondary APP1 Exif Orientation=1 must not suppress an Orientation=6 APP1 in either order.
        let mut app1_orient6 = b"Exif\0\0".to_vec();
        app1_orient6.extend_from_slice(&exif_tiff_chunk(6, false));
        for pair in [
            [app1_orient6.clone(), app1_orient1.clone()],
            [app1_orient1, app1_orient6],
        ] {
            let jpeg = inject_jpeg_app1_segments(&plain_jpeg, &pair);
            let decoded = decode_image_within_limit(&jpeg, 1024 * 1024).unwrap();
            assert_eq!((decoded.width(), decoded.height()), (32, 64));
            assert_quadrants(&decoded, [B, R, Y, G]);
        }

        // ExifIFD with TIFF IFD pointer type 13 and ASCII Orientation tag (format=2).
        let mut exif_ifd_type13 = tiff_with_exif_ifd_orientation(8, true);
        exif_ifd_type13[12..14].copy_from_slice(&13u16.to_be_bytes());
        let decoded = decode_image_within_limit(
            &encode_with_exif(&src, ImageFormat::Jpeg, exif_ifd_type13),
            1024 * 1024,
        )
        .unwrap();
        assert_eq!((decoded.width(), decoded.height()), (32, 64));
        assert_quadrants(&decoded, [G, Y, R, B]);

        let ascii_orient =
            *b"II*\x00\x08\x00\x00\x00\x01\x00\x12\x01\x02\x00\x02\x00\x00\x006\0\0\0\0\0\0\0";
        let decoded = decode_image_within_limit(
            &encode_with_exif(&src, ImageFormat::Jpeg, ascii_orient.to_vec()),
            1024 * 1024,
        )
        .unwrap();
        assert_eq!((decoded.width(), decoded.height()), (32, 64));
        assert_quadrants(&decoded, [B, R, Y, G]);
    }
}
