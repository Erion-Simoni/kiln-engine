//! PNG decoding to RGBA8.
//!
//! The decoder normalises every PNG colour type (grayscale, palette, RGB,
//! RGBA, 16-bit, `tRNS`) into straight 8-bit RGBA so the renderer only ever
//! deals with one pixel layout.

use std::io::Cursor;

/// PNG signature bytes (`\x89PNG\r\n\x1a\n`).
pub const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// Maximum accepted image dimension (in pixels) on either axis.
pub const MAX_DIMENSION: u32 = 16_384;

/// Errors produced while decoding an image asset.
#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    /// The bytes are not a PNG file.
    #[error("not a PNG file: missing signature")]
    NotPng,
    /// The PNG header is malformed.
    #[error("invalid PNG header: {0}")]
    InvalidHeader(String),
    /// The PNG data could not be decoded.
    #[error("PNG decode failed: {0}")]
    Decode(String),
    /// The image exceeds the accepted size limits.
    #[error("image too large: {width}x{height} exceeds {max}x{max}")]
    TooLarge { width: u32, height: u32, max: u32 },
}

/// A decoded image stored as straight 8-bit RGBA, row-major, top-left origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// RGBA8 pixel data, `width * height * 4` bytes.
    pub pixels: Vec<u8>,
}

impl Image {
    /// Create an image from raw RGBA8 pixels.
    ///
    /// Returns `None` when the pixel buffer length does not match
    /// `width * height * 4`.
    pub fn from_rgba(width: u32, height: u32, pixels: Vec<u8>) -> Option<Self> {
        let expected = (width as usize)
            .checked_mul(height as usize)?
            .checked_mul(4)?;
        if pixels.len() != expected {
            return None;
        }
        Some(Self {
            width,
            height,
            pixels,
        })
    }

    /// Number of bytes in one row of pixels.
    pub fn stride(&self) -> usize {
        self.width as usize * 4
    }
}

/// Whether the given bytes start with the PNG signature.
pub fn is_png(bytes: &[u8]) -> bool {
    bytes.len() >= PNG_SIGNATURE.len() && bytes[..PNG_SIGNATURE.len()] == PNG_SIGNATURE
}

/// Encode straight RGBA8 pixels into a PNG file.
///
/// This is the inverse of [`decode_png`] and is used by the editor to write
/// imported assets back to disk and by tests to build sample images.
///
/// Returns `None` when the pixel buffer does not match `width * height * 4`.
pub fn encode_png_rgba(width: u32, height: u32, pixels: &[u8]) -> Option<Vec<u8>> {
    let expected = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    if pixels.len() != expected {
        return None;
    }

    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(pixels).ok()?;
    }
    Some(out)
}

/// Decode a PNG image into straight RGBA8 pixels.
///
/// Every supported colour type is normalised: palette and grayscale images
/// are expanded, `tRNS` transparency becomes an alpha channel, and 16-bit
/// samples are truncated to 8 bits.
pub fn decode_png(bytes: &[u8]) -> Result<Image, ImageError> {
    if !is_png(bytes) {
        return Err(ImageError::NotPng);
    }

    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    // EXPAND turns palette/grayscale/tRNS into RGB(A); STRIP_16 drops the
    // low byte of 16-bit samples.
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);

    let mut reader = decoder
        .read_info()
        .map_err(|e| ImageError::InvalidHeader(e.to_string()))?;

    let (width, height) = reader.info().size();
    if width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(ImageError::TooLarge {
            width,
            height,
            max: MAX_DIMENSION,
        });
    }

    let buf_size = reader
        .output_buffer_size()
        .ok_or_else(|| ImageError::InvalidHeader("image buffer size overflow".to_string()))?;
    let mut buf = vec![0u8; buf_size];

    let output = reader
        .next_frame(&mut buf)
        .map_err(|e| ImageError::Decode(e.to_string()))?;

    let used = output.line_size * output.height as usize;
    let pixels = to_rgba8(output.color_type, &buf[..used]);

    Ok(Image {
        width: output.width,
        height: output.height,
        pixels,
    })
}

/// Convert a decoded PNG row buffer into straight RGBA8.
///
/// The decoder runs with `EXPAND`, so palette and grayscale images normally
/// arrive as RGB/RGBA already; the grayscale and indexed branches are
/// fallbacks for images that bypass that transform.
fn to_rgba8(color_type: png::ColorType, src: &[u8]) -> Vec<u8> {
    let channels = match color_type {
        png::ColorType::Rgba => 4,
        png::ColorType::Rgb => 3,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Grayscale | png::ColorType::Indexed => 1,
    };

    let mut out = Vec::with_capacity(src.len() / channels * 4);
    for px in src.chunks_exact(channels) {
        match color_type {
            png::ColorType::Rgba => out.extend_from_slice(px),
            png::ColorType::Rgb => out.extend_from_slice(&[px[0], px[1], px[2], 255]),
            png::ColorType::GrayscaleAlpha => {
                out.extend_from_slice(&[px[0], px[0], px[0], px[1]]);
            }
            png::ColorType::Grayscale | png::ColorType::Indexed => {
                out.extend_from_slice(&[px[0], px[0], px[0], 255]);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Encode a small PNG for tests with the given colour type.
    fn encode(width: u32, height: u32, color: png::ColorType, pixels: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, width, height);
            encoder.set_color(color);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(pixels).unwrap();
        }
        out
    }

    #[test]
    fn detects_png_signature() {
        assert!(is_png(&encode(1, 1, png::ColorType::Rgba, &[1, 2, 3, 4])));
        assert!(!is_png(b"not a png"));
        assert!(!is_png(&[]));
    }

    #[test]
    fn rejects_non_png_bytes() {
        let err = decode_png(b"definitely not a png").unwrap_err();
        assert!(matches!(err, ImageError::NotPng));
    }

    #[test]
    fn decodes_rgba_png() {
        let bytes = encode(
            2,
            1,
            png::ColorType::Rgba,
            &[255, 0, 0, 255, 0, 0, 255, 128],
        );
        let image = decode_png(&bytes).unwrap();
        assert_eq!(image.width, 2);
        assert_eq!(image.height, 1);
        assert_eq!(image.pixels.len(), 8);
        assert_eq!(&image.pixels[0..4], &[255, 0, 0, 255]);
        assert_eq!(&image.pixels[4..8], &[0, 0, 255, 128]);
    }

    #[test]
    fn expands_rgb_to_opaque_alpha() {
        let bytes = encode(1, 1, png::ColorType::Rgb, &[10, 20, 30]);
        let image = decode_png(&bytes).unwrap();
        assert_eq!(image.pixels, vec![10, 20, 30, 255]);
    }

    #[test]
    fn expands_grayscale_to_rgb() {
        let bytes = encode(2, 1, png::ColorType::Grayscale, &[64, 128]);
        let image = decode_png(&bytes).unwrap();
        assert_eq!(image.pixels, vec![64, 64, 64, 255, 128, 128, 128, 255]);
    }

    #[test]
    fn expands_grayscale_alpha() {
        let bytes = encode(1, 1, png::ColorType::GrayscaleAlpha, &[64, 32]);
        let image = decode_png(&bytes).unwrap();
        assert_eq!(image.pixels, vec![64, 64, 64, 32]);
    }

    #[test]
    fn decodes_palette_png() {
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, 1, 1);
            encoder.set_color(png::ColorType::Indexed);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_palette(vec![0, 0, 0, 255, 0, 0]);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&[1]).unwrap();
        }
        let image = decode_png(&out).unwrap();
        assert_eq!(image.pixels, vec![255, 0, 0, 255]);
    }

    #[test]
    fn from_rgba_validates_length() {
        assert!(Image::from_rgba(2, 2, vec![0; 16]).is_some());
        assert!(Image::from_rgba(2, 2, vec![0; 15]).is_none());
    }

    #[test]
    fn encode_png_rgba_round_trips() {
        let pixels = [255, 0, 0, 255, 0, 0, 255, 128];
        let bytes = encode_png_rgba(2, 1, &pixels).expect("encode");
        assert!(is_png(&bytes));

        let image = decode_png(&bytes).expect("decode");
        assert_eq!(image.width, 2);
        assert_eq!(image.height, 1);
        assert_eq!(image.pixels, pixels);
    }

    #[test]
    fn encode_png_rgba_validates_length() {
        assert!(encode_png_rgba(2, 2, &[0; 15]).is_none());
        assert!(encode_png_rgba(2, 2, &[0; 16]).is_some());
    }

    #[test]
    fn stride_matches_width() {
        let image = Image::from_rgba(3, 4, vec![0; 48]).unwrap();
        assert_eq!(image.stride(), 12);
    }
}
