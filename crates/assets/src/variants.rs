use std::io::Cursor;

use image::{DynamicImage, ImageReader, Limits, imageops::FilterType};
use yourdocs_core::CoreError;

use crate::ImageInfo;

/// Largest accepted picture, in pixels, whatever its file size: a tiny PNG can claim 30000 × 30000 and take gigabytes to decode.
pub const MAX_PIXELS: u64 = 40_000_000;
/// Largest accepted width or height.
pub const MAX_SIDE: u32 = 16_384;

/// The widths published for responsive images; a picture is never scaled up to reach one.
const WIDTHS: [u32; 3] = [480, 960, 1600];
const QUALITY: f32 = 80.0;

/// Refuses pictures too large to process, using the size read from the header, before any pixel is decoded.
pub fn check_size(info: &ImageInfo) -> Result<(), CoreError> {
  let pixels = u64::from(info.width) * u64::from(info.height);

  if info.width > MAX_SIDE || info.height > MAX_SIDE || pixels > MAX_PIXELS {
    return Err(CoreError::Invalid(format!(
      "the image is {} × {} pixels; the limit is {MAX_PIXELS} pixels in total and {MAX_SIDE} on a side",
      info.width, info.height
    )));
  }

  Ok(())
}

/// One scaled copy of a picture, encoded as WebP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedVariant {
  pub width: u32,
  pub height: u32,
  pub bytes: Vec<u8>,
}

/// The scaled copies of a picture and the color to show while it loads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Processed {
  pub variants: Vec<EncodedVariant>,
  /// `#rrggbb`, the average color.
  pub placeholder: String,
}

/// The widths to produce for a picture `width` pixels wide: the standard ones narrower than it, and its own width when that is not
/// larger than the biggest standard one.
fn target_widths(width: u32) -> Vec<u32> {
  let mut widths: Vec<u32> = WIDTHS.into_iter().filter(|w| *w < width).collect();

  if width <= WIDTHS[2] {
    widths.push(width);
  }

  widths
}

/// Decodes the picture and produces its WebP variants and placeholder color. Animated GIFs are left as they are (no variants).
/// CPU heavy: call it from `spawn_blocking`.
pub fn process(bytes: &[u8], info: &ImageInfo) -> Result<Processed, CoreError> {
  check_size(info)?;

  let mut reader = ImageReader::new(Cursor::new(bytes))
    .with_guessed_format()
    .map_err(|e| CoreError::Invalid(format!("the image cannot be read: {e}")))?;
  let mut limits = Limits::default();
  limits.max_image_width = Some(MAX_SIDE);
  limits.max_image_height = Some(MAX_SIDE);
  limits.max_alloc = Some(MAX_PIXELS * 4 + (64 << 20));
  reader.limits(limits);

  let image = reader.decode().map_err(|e| CoreError::Invalid(format!("the image cannot be decoded: {e}")))?;
  let placeholder = average_color(&image);

  if info.mime == "image/gif" {
    return Ok(Processed { variants: Vec::new(), placeholder });
  }

  let variants = target_widths(image.width())
    .into_iter()
    .map(|width| {
      let height = ((u64::from(image.height()) * u64::from(width)).div_ceil(u64::from(image.width()))).max(1) as u32;
      let scaled = if width == image.width() {
        image.clone()
      } else {
        image.resize_exact(width, height, FilterType::Lanczos3)
      };
      let rgba = scaled.to_rgba8();
      let bytes = webp::Encoder::from_rgba(rgba.as_raw(), rgba.width(), rgba.height()).encode(QUALITY).to_vec();

      EncodedVariant { width, height: rgba.height(), bytes }
    })
    .collect();

  Ok(Processed { variants, placeholder })
}

fn average_color(image: &DynamicImage) -> String {
  let pixel = image.resize_exact(1, 1, FilterType::Triangle).to_rgb8();
  let [r, g, b] = pixel.get_pixel(0, 0).0;

  format!("#{r:02x}{g:02x}{b:02x}")
}

#[cfg(test)]
mod tests {
  use image::{ImageFormat, RgbImage};

  use super::*;
  use crate::inspect;

  fn png(width: u32, height: u32, rgb: [u8; 3]) -> Vec<u8> {
    let mut bytes = Vec::new();
    RgbImage::from_pixel(width, height, image::Rgb(rgb)).write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png).unwrap();
    bytes
  }

  #[test]
  fn widths_never_scale_up() {
    assert_eq!(target_widths(300), [300]);
    assert_eq!(target_widths(480), [480]);
    assert_eq!(target_widths(1000), [480, 960, 1000]);
    assert_eq!(target_widths(1600), [480, 960, 1600]);
    assert_eq!(target_widths(3000), [480, 960, 1600]);
  }

  #[test]
  fn makes_webp_variants_and_an_average_color() {
    let bytes = png(1200, 600, [200, 40, 40]);
    let info = inspect(&bytes).unwrap();
    let processed = process(&bytes, &info).unwrap();

    let sizes: Vec<_> = processed.variants.iter().map(|v| (v.width, v.height)).collect();
    assert_eq!(sizes, [(480, 240), (960, 480), (1200, 600)]);
    assert!(processed.variants.iter().all(|v| v.bytes.starts_with(b"RIFF") && v.bytes[8..12] == *b"WEBP"));
    assert_eq!(processed.placeholder, "#c82828");
  }

  #[test]
  fn a_picture_that_claims_to_be_enormous_is_refused_before_decoding() {
    // A valid PNG header with a size in the billions of pixels; the rest of the file does not even exist.
    let mut bytes = png(2, 2, [0, 0, 0]);
    bytes[16..20].copy_from_slice(&30_000u32.to_be_bytes());
    bytes[20..24].copy_from_slice(&30_000u32.to_be_bytes());
    let info = inspect(&bytes).unwrap();

    assert_eq!((info.width, info.height), (30_000, 30_000));
    assert!(matches!(process(&bytes, &info), Err(CoreError::Invalid(m)) if m.contains("limit")));
  }
}
