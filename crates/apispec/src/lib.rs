//! Parsing, bundling and validation of OpenAPI documents. Everything works on `serde_json::Value`; fetching goes through a trait.

#![forbid(unsafe_code)]

mod bundle;
mod error;
mod merge;
mod pointer;
mod validate;

pub use bundle::{Fetch, bundle};
pub use error::ApiSpecError;
pub use merge::{MergeInput, MergeOptions, Merged, Operation, merge};
pub use validate::validate;

use serde_json::Value;

/// Reads a document that is JSON or YAML.
pub fn parse(bytes: &[u8]) -> Result<Value, ApiSpecError> {
  // JSON is YAML too, but its own parser gives better errors and is far faster on big specs.
  if let Ok(value) = serde_json::from_slice::<Value>(bytes) {
    return Ok(value);
  }

  serde_norway::from_slice::<Value>(bytes).map_err(|e| ApiSpecError::Parse(e.to_string()))
}

/// Parses a spec, pulls in everything it references from other files, and checks the result.
pub async fn load(bytes: &[u8], url: &str, fetch: &impl Fetch) -> Result<Value, ApiSpecError> {
  let bundled = bundle(parse(bytes)?, url, fetch).await?;
  validate(&bundled)?;

  Ok(bundled)
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;

  use serde_json::json;

  use super::*;

  struct Files(HashMap<&'static str, &'static str>);

  impl Fetch for Files {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
      self.0.get(url).map(|body| body.as_bytes().to_vec()).ok_or_else(|| "HTTP 404".to_string())
    }
  }

  type Others<'a> = &'a [(&'static str, &'static str)];

  const ROOT: &str = "https://example.com/specs/openapi.yaml";

  fn files(list: &[(&'static str, &'static str)]) -> Files {
    Files(list.iter().copied().collect())
  }

  async fn loaded(root: &str, others: Others<'_>) -> Result<Value, ApiSpecError> {
    load(root.as_bytes(), ROOT, &files(others)).await
  }

  const BASE: &str = r#"
openapi: 3.0.3
info: { title: Orders, version: "1" }
paths:
  /orders:
    get:
      responses:
        "200":
          description: ok
          content:
            application/json:
              schema: { $ref: "./schemas/order.yaml" }
"#;

  #[tokio::test]
  async fn relative_refs_become_components() {
    let spec = loaded(BASE, &[("https://example.com/specs/schemas/order.yaml", "type: object\nproperties:\n  id: { type: string }\n")])
      .await
      .unwrap();

    assert_eq!(
      spec.pointer("/paths/~1orders/get/responses/200/content/application~1json/schema/$ref"),
      Some(&json!("#/components/schemas/order"))
    );
    assert_eq!(spec.pointer("/components/schemas/order/properties/id/type"), Some(&json!("string")));
  }

  #[tokio::test]
  async fn refs_inside_other_files_follow_their_own_file_and_may_form_cycles() {
    let root = BASE.replace("./schemas/order.yaml", "./schemas/order.yaml#/Order");
    let others = [
      (
        "https://example.com/specs/schemas/order.yaml",
        "Order:\n  type: object\n  properties:\n    items: { type: array, items: { $ref: '#/Item' } }\n    ship: { $ref: './ship.json' }\nItem:\n  type: object\n  properties:\n    order: { $ref: '#/Order' }\n",
      ),
      ("https://example.com/specs/schemas/ship.json", r#"{"type":"string","enum":["a","b"]}"#),
    ];
    let spec = loaded(&root, &others).await.unwrap();

    // The cycle Order -> Item -> Order is two components pointing at each other.
    assert_eq!(spec.pointer("/components/schemas/Order/properties/items/items/$ref"), Some(&json!("#/components/schemas/Item")));
    assert_eq!(spec.pointer("/components/schemas/Item/properties/order/$ref"), Some(&json!("#/components/schemas/Order")));
    assert_eq!(spec.pointer("/components/schemas/ship/enum"), Some(&json!(["a", "b"])));
  }

  #[tokio::test]
  async fn absolute_urls_and_the_same_target_twice_give_one_component() {
    let root = r#"
openapi: 3.1.0
info: { title: T, version: "1" }
paths:
  /a:
    get:
      parameters:
        - { $ref: "https://other.example.com/common.yaml#/components/parameters/Limit" }
      responses:
        "200": { $ref: "https://other.example.com/common.yaml#/components/responses/Ok" }
  /b:
    get:
      parameters:
        - { $ref: "https://other.example.com/common.yaml#/components/parameters/Limit" }
      responses:
        "200": { $ref: "https://other.example.com/common.yaml#/components/responses/Ok" }
"#;
    let common = "components:\n  parameters:\n    Limit: { name: limit, in: query, schema: { type: integer } }\n  responses:\n    Ok: { description: fine }\n";
    let spec = loaded(root, &[("https://other.example.com/common.yaml", common)]).await.unwrap();

    assert_eq!(spec.pointer("/paths/~1a/get/parameters/0/$ref"), Some(&json!("#/components/parameters/Limit")));
    assert_eq!(spec.pointer("/paths/~1b/get/parameters/0/$ref"), Some(&json!("#/components/parameters/Limit")));
    assert_eq!(spec.pointer("/paths/~1a/get/responses/200/$ref"), Some(&json!("#/components/responses/Ok")));
    assert_eq!(spec.pointer("/components/parameters").and_then(Value::as_object).map(|m| m.len()), Some(1));
  }

  #[tokio::test]
  async fn names_do_not_clash_with_the_documents_own_components() {
    let root = r#"
openapi: 3.0.0
info: { title: T, version: "1" }
paths:
  /a:
    get:
      responses:
        "200":
          description: ok
          content: { application/json: { schema: { $ref: "./pet.yaml" } } }
components:
  schemas:
    pet: { type: string }
"#;
    let spec = loaded(root, &[("https://example.com/specs/pet.yaml", "type: object")]).await.unwrap();

    assert_eq!(spec.pointer("/components/schemas/pet/type"), Some(&json!("string")));
    assert_eq!(spec.pointer("/components/schemas/pet_2/type"), Some(&json!("object")));
  }

  #[tokio::test]
  async fn path_items_in_other_files_are_inlined_with_their_own_references() {
    let root = "openapi: 3.0.0\ninfo: { title: T, version: '1' }\npaths:\n  /pets: { $ref: './pets.yaml' }\n";
    let pets = "get:\n  responses:\n    '200':\n      description: ok\n      content: { application/json: { schema: { $ref: './pet.yaml' } } }\n";
    let spec = loaded(root, &[("https://example.com/specs/pets.yaml", pets), ("https://example.com/specs/pet.yaml", "type: object")])
      .await
      .unwrap();

    assert!(spec.pointer("/paths/~1pets/get").is_some());
    assert_eq!(spec.pointer("/components/schemas/pet/type"), Some(&json!("object")));
  }

  #[tokio::test]
  async fn json_and_yaml_both_parse() {
    let json = r#"{"openapi":"3.0.0","info":{"title":"T","version":"1"},"paths":{}}"#;

    assert!(loaded(json, &[]).await.is_ok());
    assert!(matches!(parse(b"{ not: [valid"), Err(ApiSpecError::Parse(_))));
  }

  #[tokio::test]
  async fn problems_are_reported_with_the_file() {
    let cases: [(&str, Others, &str); 5] = [
      (BASE, &[], "https://example.com/specs/schemas/order.yaml: HTTP 404"),
      (
        BASE,
        &[("https://example.com/specs/schemas/order.yaml", "{ broken: [")],
        "https://example.com/specs/schemas/order.yaml: not valid",
      ),
      (
        &BASE.replace("order.yaml\"", "order.yaml#/Nope\""),
        &[("https://example.com/specs/schemas/order.yaml", "Order: {}")],
        "does not exist in",
      ),
      (&BASE.replace("./schemas/order.yaml", "ftp://x.example/o.yaml"), &[], "not an http or https reference"),
      (&BASE.replace("./schemas/order.yaml", "./o.yaml#anchor"), &[], "not a reference this tool can follow"),
    ];

    for (root, others, expected) in cases {
      let message = loaded(root, others).await.unwrap_err().to_string();
      assert!(message.contains(expected), "{message}");
    }
  }

  #[tokio::test]
  async fn too_many_files_is_refused() {
    let mut paths = String::new();
    let mut names: Vec<(&'static str, &'static str)> = Vec::new();
    for i in 0..210 {
      let file: &'static str = Box::leak(format!("https://example.com/specs/f{i}.yaml").into_boxed_str());
      paths.push_str(&format!("  /p{i}: {{ $ref: './f{i}.yaml' }}\n"));
      names.push((file, "get: { responses: { '200': { description: ok } } }"));
    }
    let root = format!("openapi: 3.0.0\ninfo: {{ title: T, version: '1' }}\npaths:\n{paths}");

    assert!(matches!(loaded(&root, &names).await, Err(ApiSpecError::Limit(_))));
  }

  #[test]
  fn validation_table() {
    let ok30 = json!({"openapi": "3.0.3", "info": {"title": "T", "version": "1"}, "paths": {}});
    let cases = [
      (ok30.clone(), None),
      (json!({"openapi": "3.1.0", "info": {"title": "T", "version": "1"}, "components": {}}), None),
      (json!({"openapi": "3.0.3", "info": {"title": "T", "version": "1"}, "components": {}}), Some("no `paths`")),
      (json!({"swagger": "2.0", "info": {"title": "T", "version": "1"}, "paths": {}}), Some("Swagger 2.0 is not supported")),
      (json!({"openapi": "4.0.0", "info": {"title": "T", "version": "1"}, "paths": {}}), Some("OpenAPI 4.0.0 is not supported")),
      (json!({"openapi": "3.0.3", "info": {"title": "T"}, "paths": {}}), Some("`info.version` is missing")),
      (json!({"title": "no version"}), Some("no `openapi` version")),
      (
        json!({"openapi": "3.0.3", "info": {"title": "T", "version": "1"}, "paths": {"/a": {"get": {"x": {"$ref": "#/components/schemas/Gone"}}}}}),
        Some("does not lead to anything"),
      ),
      (
        json!({"openapi": "3.0.3", "info": {"title": "T", "version": "1"}, "paths": {}, "components": {"schemas": {"A": {"$ref": "#/components/schemas/B"}, "B": {"$ref": "#/components/schemas/A"}}}}),
        Some("in a loop"),
      ),
      (
        json!({"openapi": "3.0.3", "info": {"title": "T", "version": "1"}, "paths": {"/a": {"get": {"x": {"$ref": "./other.yaml"}}}}}),
        Some("was not resolved"),
      ),
      // Example data may contain anything, a `$ref` key included.
      (
        json!({"openapi": "3.0.3", "info": {"title": "T", "version": "1"}, "paths": {}, "components": {"schemas": {"A": {"example": {"$ref": "#/nowhere"}}}}}),
        None,
      ),
    ];

    for (doc, expected) in cases {
      match (validate(&doc), expected) {
        (Ok(()), None) => {}
        (Err(e), Some(text)) => assert!(e.to_string().contains(text), "{e}"),
        (got, expected) => panic!("{got:?} but expected {expected:?}"),
      }
    }
  }
}
