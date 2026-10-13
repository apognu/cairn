use serde_json::Value;

use crate::{ApiSpecError, pointer::resolve};

/// Longest chain of references that only point at one another before it counts as a loop.
const MAX_CHAIN: usize = 32;

/// Checks that a bundled document is OpenAPI 3.0 or 3.1 with what Scalar and the merge need, and that every reference leads somewhere.
/// It is not a schema validation: a spec with odd keywords passes, one that cannot be read does not.
pub fn validate(doc: &Value) -> Result<(), ApiSpecError> {
  let invalid = |message: String| Err(ApiSpecError::Invalid(message));

  if let Some(swagger) = doc.get("swagger") {
    return invalid(format!("Swagger {} is not supported: convert the spec to OpenAPI 3", swagger.as_str().unwrap_or("2.0")));
  }

  let Some(version) = doc.get("openapi").and_then(Value::as_str) else {
    return invalid("the document has no `openapi` version".into());
  };
  let v31 = version.starts_with("3.1.");
  if !v31 && !version.starts_with("3.0.") {
    return invalid(format!("OpenAPI {version} is not supported: use 3.0.x or 3.1.x"));
  }

  for field in ["title", "version"] {
    if !doc.pointer(&format!("/info/{field}")).is_some_and(Value::is_string) {
      return invalid(format!("`info.{field}` is missing"));
    }
  }

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
