use serde_json::Value;

/// The value a JSON pointer (`/components/schemas/Pet`, with `~0` and `~1` escapes) leads to. An empty pointer is the whole document.
pub(crate) fn resolve<'a>(doc: &'a Value, pointer: &str) -> Option<&'a Value> {
  if pointer.is_empty() {
    return Some(doc);
  }

  let mut at = doc;
  for part in pointer.strip_prefix('/')?.split('/') {
    let key = part.replace("~1", "/").replace("~0", "~");
    at = match at {
      Value::Object(map) => map.get(&key)?,
      Value::Array(items) => items.get(key.parse::<usize>().ok()?)?,
      _ => return None,
    };
  }

  Some(at)
}

/// Splits a `$ref` into the file part and the pointer: `./a.yaml#/Pet` is `("./a.yaml", "/Pet")`. The pointer is percent-decoded, as
/// the fragment of a URI is.
pub(crate) fn split_ref(reference: &str) -> (&str, String) {
  match reference.split_once('#') {
    Some((file, fragment)) => (file, decode(fragment)),
    None => (reference, String::new()),
  }
}

fn decode(fragment: &str) -> String {
  let bytes = fragment.as_bytes();
  let mut out = Vec::with_capacity(bytes.len());
  let mut i = 0;

  while i < bytes.len() {
    let hex = (bytes[i] == b'%').then(|| fragment.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok())).flatten();
    match hex {
      Some(byte) => {
        out.push(byte);
        i += 3;
      }
      None => {
        out.push(bytes[i]);
        i += 1;
      }
    }
  }

  String::from_utf8_lossy(&out).into_owned()
}
