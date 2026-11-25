use std::io::Cursor;

use image::{
    DynamicImage, ExtendedColorType, ImageDecoder, ImageEncoder, ImageFormat, ImageReader, Rgb,
    RgbImage, codecs::jpeg::JpegEncoder, metadata::Orientation,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuadrantColor {
    Red,
    Green,
    Blue,
    Yellow,
}

/// Generate valid PNG bytes for integration and E2E tests.
pub fn png_bytes_for_test(width: u32, height: u32) -> Vec<u8> {
    let img = DynamicImage::new_rgb8(width, height);
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, ImageFormat::Png).unwrap();
    out.into_inner()
}

fn quadrant_rgb_for_test(width: u32, height: u32) -> RgbImage {
    let mut rgb = RgbImage::new(width, height);
    for (x, y, px) in rgb.enumerate_pixels_mut() {
        *px = match (x < width / 2, y < height / 2) {
            (true, true) => Rgb([240, 20, 20]),
            (false, true) => Rgb([20, 240, 20]),
            (true, false) => Rgb([20, 20, 240]),
            (false, false) => Rgb([240, 240, 20]),
        };
    }
    rgb
}

fn minimal_exif_tiff(orientation: Option<u16>) -> Vec<u8> {
    let mut exif = Vec::from(*b"II*\x00");
    exif.extend_from_slice(&8u32.to_le_bytes());
    if let Some(orient) = orientation {
        exif.extend_from_slice(&1u16.to_le_bytes());
        exif.extend_from_slice(&0x0112u16.to_le_bytes());
        exif.extend_from_slice(&3u16.to_le_bytes());
        exif.extend_from_slice(&1u32.to_le_bytes());
        exif.extend_from_slice(&orient.to_le_bytes());
        exif.extend_from_slice(&0u16.to_le_bytes());
    } else {
        exif.extend_from_slice(&1u16.to_le_bytes());
        exif.extend_from_slice(&0x010Fu16.to_le_bytes());
        exif.extend_from_slice(&2u16.to_le_bytes());
        exif.extend_from_slice(&4u32.to_le_bytes());
        exif.extend_from_slice(b"Cam\0");
    }
    exif.extend_from_slice(&0u32.to_le_bytes());
    exif
}

fn inject_app1_segments(jpeg: &[u8], segments: &[Vec<u8>]) -> Vec<u8> {
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

/// Generate a JPEG whose 4 quadrants are TL=Red, TR=Green, BL=Blue, BR=Yellow with EXIF orientation.
pub fn quadrant_jpeg_with_exif_for_test(width: u32, height: u32, exif_orientation: u16) -> Vec<u8> {
    let rgb = quadrant_rgb_for_test(width, height);
    let mut out = Cursor::new(Vec::new());
    let mut enc = JpegEncoder::new_with_quality(&mut out, 95);
    enc.set_exif_metadata(minimal_exif_tiff(Some(exif_orientation)))
        .unwrap();
    enc.write_image(&rgb, width, height, ExtendedColorType::Rgb8)
        .unwrap();
    out.into_inner()
}

/// Generate a JPEG with a primary EXIF orientation followed by a secondary Exif APP1 without orientation.
pub fn quadrant_jpeg_with_shadowed_exif_for_test(
    width: u32,
    height: u32,
    exif_orientation: u16,
) -> Vec<u8> {
    let rgb = quadrant_rgb_for_test(width, height);
    let mut out = Cursor::new(Vec::new());
    let enc = JpegEncoder::new_with_quality(&mut out, 95);
    enc.write_image(&rgb, width, height, ExtendedColorType::Rgb8)
        .unwrap();

    let mut primary = b"Exif\0\0".to_vec();
    primary.extend_from_slice(&minimal_exif_tiff(Some(exif_orientation)));
    let mut secondary = b"Exif\0\0".to_vec();
    secondary.extend_from_slice(&minimal_exif_tiff(None));
    inject_app1_segments(&out.into_inner(), &[primary, secondary])
}

/// Generate a JPEG whose orientation is stored only in XMP `tiff:Orientation`.
pub fn quadrant_jpeg_with_xmp_orientation_for_test(
    width: u32,
    height: u32,
    orientation: u16,
) -> Vec<u8> {
    let rgb = quadrant_rgb_for_test(width, height);
    let mut out = Cursor::new(Vec::new());
    let enc = JpegEncoder::new_with_quality(&mut out, 95);
    enc.write_image(&rgb, width, height, ExtendedColorType::Rgb8)
        .unwrap();

    let mut xmp = b"http://ns.adobe.com/xap/1.0/\0".to_vec();
    xmp.extend_from_slice(
        format!(r#"<rdf:Description tiff:Orientation="{orientation}"/>"#).as_bytes(),
    );
    inject_app1_segments(&out.into_inner(), &[xmp])
}

/// Assert a stored WebP has no EXIF metadata and return its dimensions and `[TL, TR, BL, BR]` quadrants.
pub fn inspect_normalized_webp_for_test(webp_bytes: &[u8]) -> (u32, u32, [QuadrantColor; 4]) {
    let mut decoder = ImageReader::new(Cursor::new(webp_bytes))
        .with_guessed_format()
        .unwrap()
        .into_decoder()
        .unwrap();
    assert_eq!(decoder.exif_metadata().unwrap(), None);
    assert_eq!(decoder.orientation().unwrap(), Orientation::NoTransforms);

    let img = DynamicImage::from_decoder(decoder).unwrap().to_rgb8();
    let (w, h) = (img.width(), img.height());
    let classify = |x: u32, y: u32| {
        let [r, g, b] = img.get_pixel(x, y).0;
        match (r > 150, g > 150, b > 150) {
            (true, false, false) => QuadrantColor::Red,
            (false, true, false) => QuadrantColor::Green,
            (false, false, true) => QuadrantColor::Blue,
            (true, true, false) => QuadrantColor::Yellow,
            _ => panic!("unexpected color at ({x}, {y}): [{r}, {g}, {b}]"),
        }
    };
    (
        w,
        h,
        [
            classify(w / 4, h / 4),
            classify(3 * w / 4, h / 4),
            classify(w / 4, 3 * h / 4),
            classify(3 * w / 4, 3 * h / 4),
        ],
    )
}

/// Assert a WebP byte buffer contains no EXIF metadata and has expected dimensions.
pub fn assert_webp_without_exif_for_test(
    webp_bytes: &[u8],
    expected_width: u32,
    expected_height: u32,
) {
    let mut decoder = ImageReader::new(Cursor::new(webp_bytes))
        .with_guessed_format()
        .unwrap()
        .into_decoder()
        .unwrap();
    assert_eq!(decoder.exif_metadata().unwrap(), None);
    assert_eq!(decoder.orientation().unwrap(), Orientation::NoTransforms);
    assert_eq!(decoder.dimensions(), (expected_width, expected_height));
}
