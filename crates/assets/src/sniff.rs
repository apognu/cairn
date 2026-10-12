use sha2::{Digest, Sha256};
use yourdocs_core::CoreError;

/// What an uploaded file turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageInfo {
  pub mime: &'static str,
  /// File extension without the dot, for the published URL.
  pub extension: &'static str,
  pub width: u32,
  pub height: u32,
}

/// Identifies an image by its first bytes, never by the name or type the client claims, and reads its size from the header.
///
/// SVG is refused for now: it can carry scripts, and sanitizing it is its own task.
pub fn inspect(bytes: &[u8]) -> Result<ImageInfo, CoreError> {
  let (mime, extension) = match imagesize::image_type(bytes) {
    Ok(imagesize::ImageType::Png) => ("image/png", "png"),
    Ok(imagesize::ImageType::Jpeg) => ("image/jpeg", "jpg"),
    Ok(imagesize::ImageType::Gif) => ("image/gif", "gif"),
    Ok(imagesize::ImageType::Webp) => ("image/webp", "webp"),
    _ => return Err(CoreError::Invalid("only PNG, JPEG, GIF and WebP images are accepted".into())),
  };

  let size = imagesize::blob_size(bytes).map_err(|_| CoreError::Invalid("the image header is damaged".into()))?;

  Ok(ImageInfo {
    mime,
    extension,
    width: size.width as u32,
    height: size.height as u32,
  })
}

pub fn sha256_hex(bytes: &[u8]) -> String {
  Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  // 1x1 PNG
  const PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89,
  ];

  #[test]
  fn reads_type_and_size_from_the_bytes() {
    let info = inspect(PNG).unwrap();

    assert_eq!((info.mime, info.extension, info.width, info.height), ("image/png", "png", 1, 1));
  }

  #[test]
  fn refuses_anything_that_is_not_a_known_image() {
    for bytes in [&b"<svg xmlns='http://www.w3.org/2000/svg'><script>alert(1)</script></svg>"[..], b"%PDF-1.7", b""] {
      assert!(matches!(inspect(bytes), Err(CoreError::Invalid(_))));
    }
  }

  #[test]
  fn hashes_like_sha256() {
    assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
  }
}
