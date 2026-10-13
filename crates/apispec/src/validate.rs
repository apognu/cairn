use serde_json::{Value, json};

use crate::{ApiSpecError, pointer::resolve};

/// Longest chain of references that only point at one another before it counts as a loop.
const MAX_CHAIN: usize = 32;

/// Fills in what a spec in the wild often leaves out, so it can still be read: no `openapi` version is taken for 3.0.3, and a missing
/// title or version gets a placeholder.
pub fn normalize(doc: &mut Value) {
  let Some(root) = doc.as_object_mut() else { return };

  if !root.contains_key("openapi") && !root.contains_key("swagger") {
    root.insert("openapi".into(), "3.0.3".into());
  }
  let info = root.entry("info").or_insert_with(|| json!({}));
  if let Some(info) = info.as_object_mut() {
    info.entry("title").or_insert_with(|| "API".into());
    info.entry("version").or_insert_with(|| "0.0.0".into());
  }
}

/// Checks that a bundled document can be merged and shown, and that every reference leads somewhere. It is not a schema validation: a
/// spec with odd keywords passes, one that cannot be read does not.
pub fn validate(doc: &Value) -> Result<(), ApiSpecError> {
  let invalid = |message: String| Err(ApiSpecError::Invalid(message));

  if let Some(swagger) = doc.get("swagger") {
    return invalid(format!("Swagger {} is not supported: convert the spec to OpenAPI 3", swagger.as_str().unwrap_or("2.0")));
  }

  let version = doc.get("openapi").and_then(Value::as_str).unwrap_or("3.0.3");
  if !version.starts_with("3.") {
    return invalid(format!("OpenAPI {version} is not supported: use a 3.x version"));
  }
  let v31 = !version.starts_with("3.0.");

  // 3.1 lets a document hold only components or webhooks.
  let has = |key: &str| doc.get(key).is_some_and(Value::is_object);
  if !(has("paths") || v31 && (has("components") || has("webhooks"))) {
    return invalid(
      if v31 {
        "the document has no `paths`, `components` or `webhooks`"
      } else {
        "the document has no `paths`"
      }
      .into(),
    );
  }

  Ok(())
}

/// Checks that every `#/…` reference of a (merged) document leads somewhere. A source may use the components of another source, so this
/// runs on the result of the merge and not on each source.
pub fn validate_refs(doc: &Value) -> Result<(), ApiSpecError> {
  check_refs(doc, doc)
}

fn check_refs(doc: &Value, at: &Value) -> Result<(), ApiSpecError> {
  match at {
    Value::Object(map) => {
      if let Some(Value::String(reference)) = map.get("$ref") {
        check_ref(doc, reference)?;
      }
      for (key, child) in map {
        // What an example or a default holds is data, which may contain anything.
        let data = matches!(key.as_str(), "example" | "default" | "enum" | "const") || key == "examples" && child.is_array();
        if !data {
          check_refs(doc, child)?;
        }
      }
    }
    Value::Array(items) => {
      for item in items {
        check_refs(doc, item)?;
      }
    }
    _ => {}
  }

  Ok(())
}

fn check_ref(doc: &Value, reference: &str) -> Result<(), ApiSpecError> {
  let mut current = reference.to_string();

  for _ in 0..MAX_CHAIN {
    let Some(pointer) = current.strip_prefix('#') else {
      return Err(ApiSpecError::Invalid(format!("the reference {current} was not resolved: only references inside the document are kept")));
    };
    let Some(target) = resolve(doc, &decode(pointer)) else {
      return Err(ApiSpecError::Invalid(format!("the reference {reference} does not lead to anything")));
    };

    match target.get("$ref").and_then(Value::as_str) {
      Some(next) if target.as_object().is_some_and(|m| m.len() == 1) => current = next.to_string(),
      _ => return Ok(()),
    }
  }

  Err(ApiSpecError::Invalid(format!("the reference {reference} only leads to other references, in a loop")))
}

fn decode(pointer: &str) -> String {
  crate::pointer::split_ref(&format!("#{pointer}")).1
}
