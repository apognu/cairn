use std::io::Cursor;

use image::{DynamicImage, ImageDecoder, ImageReader, Limits, imageops::FilterType};
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
  /// The size as displayed: a photo turned by its EXIF orientation has its sides swapped compared to the file header.
  pub width: u32,
  pub height: u32,
  pub variants: Vec<EncodedVariant>,
  /// `#rrggbb`, the average color; none for a picture with transparent parts, which would show the color through them for good.
  pub placeholder: Option<String>,
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

  let invalid = |e: image::ImageError| CoreError::Invalid(format!("the image cannot be decoded: {e}"));
  let mut decoder = reader.into_decoder().map_err(invalid)?;
  // Phones store photos as shot and an EXIF tag saying how to turn them; the WebP variants carry no EXIF, so they are turned here.
  let orientation = decoder.orientation().map_err(invalid)?;
  let mut image = DynamicImage::from_decoder(decoder).map_err(invalid)?;
  image.apply_orientation(orientation);

  let placeholder = average_color(&image);
  let (width, height) = (image.width(), image.height());

  if info.mime == "image/gif" {
    return Ok(Processed {
      width,
      height,
      variants: Vec::new(),
      placeholder,
    });
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

  Ok(Processed { width, height, variants, placeholder })
}

fn average_color(image: &DynamicImage) -> Option<String> {
  let pixel = image.resize_exact(1, 1, FilterType::Triangle).to_rgba8();
  let [r, g, b, a] = pixel.get_pixel(0, 0).0;

  // Some transparency, averaged with the rest: the placeholder would stay visible behind the transparent parts once loaded.
  (a >= 250).then(|| format!("#{r:02x}{g:02x}{b:02x}"))
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
    assert_eq!(processed.placeholder.as_deref(), Some("#c82828"));
  }

  #[test]
  fn a_photo_is_turned_the_way_its_exif_says() {
    let mut jpeg = Vec::new();
    RgbImage::from_pixel(40, 20, image::Rgb([90, 90, 90])).write_to(&mut Cursor::new(&mut jpeg), ImageFormat::Jpeg).unwrap();
    // An APP1 segment right after the start marker: EXIF, little-endian TIFF, one entry saying "orientation 6" (turn 90° clockwise).
    let exif: &[u8] = &[
      0xFF, 0xE1, 0x00, 0x22, b'E', b'x', b'i', b'f', 0, 0, b'I', b'I', 0x2A, 0, 8, 0, 0, 0, 1, 0, 0x12, 0x01, 3, 0, 1, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0,
    ];
    jpeg.splice(2..2, exif.iter().copied());
    let info = inspect(&jpeg).unwrap();

    let processed = process(&jpeg, &info).unwrap();
    assert_eq!((processed.width, processed.height), (20, 40));
    assert_eq!(processed.variants.iter().map(|v| (v.width, v.height)).collect::<Vec<_>>(), [(20, 40)]);
  }

  #[test]
  fn a_transparent_picture_has_no_placeholder() {
    let mut bytes = Vec::new();
    image::RgbaImage::from_fn(4, 4, |x, _| image::Rgba([0, 0, 0, if x < 2 { 0 } else { 255 }]))
      .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
      .unwrap();
    let info = inspect(&bytes).unwrap();

    assert_eq!(process(&bytes, &info).unwrap().placeholder, None);
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
