use std::collections::{HashMap, HashSet};

use serde_json::{Map, Value, json};
use yourdocs_core::Operation;

use crate::ApiSpecError;

const METHODS: [&str; 8] = ["get", "put", "post", "delete", "options", "head", "patch", "trace"];

/// One bundled spec, with the name it goes by in messages and the prefix that keeps its names apart from the others'.
pub struct MergeInput {
  pub label: String,
  pub spec: Value,
  pub prefix: String,
}

/// What the page's author set by hand; anything left out comes from the first source.
#[derive(Debug, Default, Clone)]
pub struct MergeOptions {
  pub title: Option<String>,
  pub version: Option<String>,
  pub description: Option<String>,
  pub servers: Vec<String>,
}

#[derive(Debug)]
pub struct Merged {
  pub spec: Value,
  pub operations: Vec<Operation>,
}

/// Combines specs into one, in order. A prefix is put before the tags (`Webhooks Retry`, and `Webhooks` for an operation without any),
/// the operation IDs and the component names (`Webhooks_Event`) of its source. Sources that describe the same method and path, or share an
/// operation ID, a webhook, or a component name with different content, are a conflict that names both. A prefix settles the clashes of IDs, tags
/// and components, never two descriptions of one endpoint.
pub fn merge(mut inputs: Vec<MergeInput>, options: &MergeOptions) -> Result<Merged, ApiSpecError> {
  if inputs.is_empty() {
    return Err(ApiSpecError::Invalid("an API reference needs at least one source".into()));
  }

  let several = inputs.len() > 1;
  let mut state = State::default();

  for input in &mut inputs {
    if !input.prefix.is_empty() {
      prefix_names(&mut input.spec, &input.prefix);
    }
    if several {
      push_security_down(&mut input.spec);
    }
    state.add(input)?;
  }

  let first = &inputs[0].spec;
  let mut info = first.get("info").cloned().unwrap_or_else(|| json!({}));
  for (key, value) in [("title", &options.title), ("version", &options.version), ("description", &options.description)] {
    if let (Some(value), Some(info)) = (value, info.as_object_mut()) {
      info.insert(key.into(), Value::String(value.clone()));
    }
  }

  let mut spec = Map::new();
  // The highest version wins, so a 3.1 document is not downgraded by a 3.0 one next to it.
  let openapi = inputs.iter().filter_map(|i| i.spec.get("openapi").and_then(Value::as_str)).max().unwrap_or("3.0.3");
  spec.insert("openapi".into(), openapi.into());
  spec.insert("info".into(), info);

  if !options.servers.is_empty() {
    spec.insert("servers".into(), options.servers.iter().map(|url| json!({ "url": url })).collect());
  } else if let Some(servers) = first.get("servers") {
    spec.insert("servers".into(), servers.clone());
  }
  if !state.tags.is_empty() {
    spec.insert("tags".into(), Value::Array(state.tags));
  }
  spec.insert("paths".into(), Value::Object(state.paths));
  if !state.webhooks.is_empty() {
    spec.insert("webhooks".into(), Value::Object(state.webhooks));
  }
  if !state.components.is_empty() {
    spec.insert("components".into(), Value::Object(state.components.into_iter().map(|(k, v)| (k, Value::Object(v))).collect()));
  }
  if let (false, Some(security)) = (several, first.get("security")) {
    spec.insert("security".into(), security.clone());
  }

  Ok(Merged {
    spec: Value::Object(spec),
    operations: state.operations,
  })
}

#[derive(Default)]
struct State {
  paths: Map<String, Value>,
  webhooks: Map<String, Value>,
  components: std::collections::BTreeMap<String, Map<String, Value>>,
  tags: Vec<Value>,
  operations: Vec<Operation>,
  /// Which source holds each (method, path), operation ID, component, webhook and tag.
  owners: HashMap<String, String>,
  tag_names: HashSet<String>,
}

impl State {
  fn add(&mut self, input: &MergeInput) -> Result<(), ApiSpecError> {
    let spec = &input.spec;
    let label = &input.label;

    if let Some(Value::Object(paths)) = spec.get("paths") {
      for (path, item) in paths {
        self.add_path(label, path, item)?;
      }
    }

    if let Some(Value::Object(webhooks)) = spec.get("webhooks") {
      for (name, item) in webhooks {
        if let Some(first) = self.owners.insert(format!("webhook {name}"), label.clone()) {
          return Err(conflict(format!("duplicate webhook {name}"), &first, label));
        }
        self.webhooks.insert(name.clone(), item.clone());
      }
    }

    if let Some(Value::Object(components)) = spec.get("components") {
      for (section, entries) in components {
        let Value::Object(entries) = entries else { continue };

        for (name, value) in entries {
          let target = self.components.entry(section.clone()).or_default();
          match target.get(name) {
            Some(existing) if existing == value => {}
            Some(_) => {
              let first = self.owners.get(&format!("component {section}/{name}")).cloned().unwrap_or_default();
              return Err(conflict(format!("{section}/{name} is defined differently"), &first, label).with_hint());
            }
            None => {
              target.insert(name.clone(), value.clone());
              self.owners.insert(format!("component {section}/{name}"), label.clone());
            }
          }
        }
      }
    }

    if let Some(Value::Array(tags)) = spec.get("tags") {
      for tag in tags {
        if let Some(name) = tag.get("name").and_then(Value::as_str)
          && self.tag_names.insert(name.to_string())
        {
          self.tags.push(tag.clone());
        }
      }
    }

    Ok(())
  }

  fn add_path(&mut self, label: &str, path: &str, item: &Value) -> Result<(), ApiSpecError> {
    let Value::Object(item) = item else { return Ok(()) };
    let existed = self.paths.contains_key(path);
    let mut item = item.clone();

    // Another source already holds this path: what each source sets for the whole path only applies to its own operations, so it moves
    // onto them, on both sides.
    if existed {
      if let Some(Value::Object(held)) = self.paths.get_mut(path) {
        push_path_level_down(held);
      }
      push_path_level_down(&mut item);
    }

    for method in METHODS {
      let Some(Value::Object(operation)) = item.get_mut(method) else { continue };
      let upper = method.to_uppercase();

      if let Some(first) = self.owners.insert(format!("operation {upper} {path}"), label.to_string()) {
        return Err(conflict(format!("duplicate operation {upper} {path}"), &first, label));
      }
      let id = operation.get("operationId").and_then(Value::as_str).map(str::to_string);
      if let Some(id) = &id
        && let Some(first) = self.owners.insert(format!("operationId {id}"), label.to_string())
      {
        return Err(conflict(format!("operationId {id} is used by more than one operation"), &first, label).with_hint());
      }

      let text = |key: &str| operation.get(key).and_then(Value::as_str).map(str::to_string);
      self.operations.push(Operation {
        method: upper,
        path: path.to_string(),
        summary: text("summary"),
        description: text("description").map(|d| d.chars().take(500).collect()),
        tags: operation
          .get("tags")
          .and_then(Value::as_array)
          .map(|t| t.iter().filter_map(Value::as_str).map(str::to_string).collect())
          .unwrap_or_default(),
        operation_id: id,
      });
    }

    match self.paths.get_mut(path) {
      Some(Value::Object(held)) if existed => {
        for (key, value) in item {
          held.entry(key).or_insert(value);
        }
      }
      _ => {
        self.paths.insert(path.to_string(), Value::Object(item));
      }
    }

    Ok(())
  }
}

/// Moves the path-level parameters and servers of a path item onto each of its operations. An operation's own parameter wins over a
/// path-level one with the same name and location, as it would have.
fn push_path_level_down(item: &mut Map<String, Value>) {
  let parameters = item.remove("parameters");
  let servers = item.remove("servers");

  for method in METHODS {
    let Some(Value::Object(operation)) = item.get_mut(method) else { continue };

    if let Some(Value::Array(shared)) = &parameters {
      let own = operation.entry("parameters").or_insert_with(|| Value::Array(Vec::new()));
      if let Value::Array(own) = own {
        let key = |p: &Value| (p.get("name").cloned(), p.get("in").cloned(), p.get("$ref").cloned());
        let overridden: Vec<_> = own.iter().map(key).collect();
        let inherited: Vec<Value> = shared.iter().filter(|p| !overridden.contains(&key(p))).cloned().collect();
        own.splice(0..0, inherited);
      }
    }
    if let Some(servers) = &servers {
      operation.entry("servers").or_insert_with(|| servers.clone());
    }
  }
}

fn conflict(what: String, first: &str, second: &str) -> ApiSpecError {
  ApiSpecError::Conflict(format!("{what}: {first} and {second}"))
}

impl ApiSpecError {
  /// A conflict a prefix can resolve says so; one that is the same endpoint twice does not.
  fn with_hint(self) -> Self {
    match self {
      ApiSpecError::Conflict(message) => ApiSpecError::Conflict(format!("{message}. Give one of the sources a prefix")),
      other => other,
    }
  }
}

/// Puts the prefix on the tags, the operation IDs and the components of one spec, and makes every reference and security requirement follow.
fn prefix_names(spec: &mut Value, prefix: &str) {
  let mut renamed: HashMap<(String, String), String> = HashMap::new();
  let mut schemes: HashMap<String, String> = HashMap::new();

  if let Some(Value::Object(components)) = spec.get_mut("components") {
    for (section, entries) in components.iter_mut() {
      let Value::Object(entries) = entries else { continue };
      let old = std::mem::take(entries);

      for (name, value) in old {
        let new = format!("{prefix}_{name}");
        if section == "securitySchemes" {
          schemes.insert(name.clone(), new.clone());
        }
        renamed.insert((section.clone(), name), new.clone());
        entries.insert(new, value);
      }
    }
  }

  rewrite_refs(spec, &renamed);

  let tag = |name: &str| format!("{prefix} {name}");
  if let Some(Value::Array(tags)) = spec.get_mut("tags") {
    for t in tags {
      if let Some(Value::String(name)) = t.get_mut("name") {
        *name = tag(name);
      }
    }
  }

  let visit_security = |value: &mut Value| {
    if let Some(Value::Array(requirements)) = value.get_mut("security") {
      for requirement in requirements {
        if let Value::Object(map) = requirement {
          *map = std::mem::take(map).into_iter().map(|(k, v)| (schemes.get(&k).cloned().unwrap_or(k), v)).collect();
        }
      }
    }
  };
  visit_security(spec);

  for key in ["paths", "webhooks"] {
    let Some(Value::Object(items)) = spec.get_mut(key) else { continue };

    for item in items.values_mut() {
      for method in METHODS {
        let Some(operation) = item.get_mut(method) else { continue };
        visit_security(operation);

        let Some(operation) = operation.as_object_mut() else { continue };
        if let Some(Value::String(id)) = operation.get_mut("operationId") {
          *id = format!("{prefix}_{id}");
        }
        let tags = match operation.get("tags") {
          Some(Value::Array(tags)) if !tags.is_empty() => tags.iter().filter_map(Value::as_str).map(tag).collect(),
          _ => vec![prefix.to_string()],
        };
        operation.insert("tags".into(), tags.into_iter().map(Value::String).collect());
      }
    }
  }
}

fn rewrite_refs(value: &mut Value, renamed: &HashMap<(String, String), String>) {
  match value {
    Value::Object(map) => {
      if let Some(Value::String(reference)) = map.get_mut("$ref") {
        if let Some(new) = renamed_ref(reference, renamed) {
          *reference = new;
        }
        return;
      }
      for (key, child) in map {
        if !matches!(key.as_str(), "example" | "default" | "enum" | "const") {
          rewrite_refs(child, renamed);
        }
      }
    }
    Value::Array(items) => {
      for item in items {
        rewrite_refs(item, renamed);
      }
    }
    _ => {}
  }
}

fn renamed_ref(reference: &str, renamed: &HashMap<(String, String), String>) -> Option<String> {
  let rest = reference.strip_prefix("#/components/")?;
  let (section, rest) = rest.split_once('/')?;
  let (name, tail) = rest.split_once('/').map_or((rest, ""), |(n, t)| (n, t));
  let new = renamed.get(&(section.to_string(), name.replace("~1", "/").replace("~0", "~")))?;
  let new = new.replace('~', "~0").replace('/', "~1");

  Some(if tail.is_empty() {
    format!("#/components/{section}/{new}")
  } else {
    format!("#/components/{section}/{new}/{tail}")
  })
}

/// With several sources the top-level `security` of one would apply to the others' operations too, so each source's goes onto its own.
fn push_security_down(spec: &mut Value) {
  let Some(root) = spec.as_object_mut() else { return };
  let Some(security) = root.remove("security") else { return };

  for key in ["paths", "webhooks"] {
    let Some(Value::Object(items)) = root.get_mut(key) else { continue };

    for item in items.values_mut() {
      for method in METHODS {
        if let Some(Value::Object(operation)) = item.get_mut(method) {
          operation.entry("security").or_insert_with(|| security.clone());
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn spec(title: &str, version: &str, paths: Value, components: Value) -> Value {
    json!({ "openapi": "3.0.3", "info": { "title": title, "version": version }, "paths": paths, "components": components })
  }

  fn input(label: &str, prefix: &str, spec: Value) -> MergeInput {
    MergeInput {
      label: label.into(),
      spec,
      prefix: prefix.into(),
    }
  }

  fn orders() -> Value {
    spec(
      "Orders",
      "1.4.2",
      json!({ "/orders": { "get": { "operationId": "list", "tags": ["Orders"], "summary": "List orders", "responses": { "200": { "description": "ok", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Page" } } } } } } } }),
      json!({ "schemas": { "Page": { "type": "object", "properties": { "n": { "type": "integer" } } } } }),
    )
  }

  fn billing() -> Value {
    spec(
      "Billing",
      "2.0.1",
      json!({ "/invoices": { "get": { "operationId": "list", "tags": ["Invoices"], "responses": { "200": { "description": "ok", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Page" } } } } } } } }),
      json!({ "schemas": { "Page": { "type": "object", "properties": { "total": { "type": "number" } } } } }),
    )
  }

  fn merged(inputs: Vec<MergeInput>) -> Result<Merged, ApiSpecError> {
    merge(inputs, &MergeOptions::default())
  }

  #[test]
  fn different_paths_merge_and_the_first_source_gives_the_info() {
    let a = spec("Orders", "1", json!({ "/orders": { "get": { "tags": ["O"], "responses": {} } } }), json!({}));
    let b = spec("Billing", "2", json!({ "/invoices": { "post": { "summary": "Create", "responses": {} } } }), json!({}));
    let out = merged(vec![input("orders", "", a), input("billing", "", b)]).unwrap();

    assert_eq!(out.spec.pointer("/info/title"), Some(&json!("Orders")));
    assert!(out.spec.pointer("/paths/~1orders/get").is_some() && out.spec.pointer("/paths/~1invoices/post").is_some());
    assert_eq!(out.operations.iter().map(|o| format!("{} {}", o.method, o.path)).collect::<Vec<_>>(), ["GET /orders", "POST /invoices"]);
    assert_eq!(out.operations[1].summary.as_deref(), Some("Create"));
  }

  #[test]
  fn the_same_method_and_path_names_both_sources_and_a_prefix_does_not_help() {
    let a = spec("A", "1", json!({ "/pets": { "get": { "responses": {} } } }), json!({}));
    let b = spec("B", "2", json!({ "/pets": { "get": { "responses": {} } } }), json!({}));

    for prefix in ["", "b"] {
      let error = merged(vec![input("orders (v1.4.2)", "", a.clone()), input("billing (v2.0.1)", prefix, b.clone())]).err().unwrap();
      assert_eq!(error.to_string(), "duplicate operation GET /pets: orders (v1.4.2) and billing (v2.0.1)");
    }
  }

  #[test]
  fn paths_shared_between_sources_merge_their_methods_and_keep_their_parameters() {
    let a = json!({ "openapi": "3.0.3", "info": { "title": "A", "version": "1" }, "paths": { "/pets/{id}": { "parameters": [{ "name": "id", "in": "path" }], "get": { "responses": {} } } } });
    let b = json!({ "openapi": "3.0.3", "info": { "title": "B", "version": "1" }, "paths": { "/pets/{id}": { "parameters": [{ "name": "pet", "in": "path" }], "delete": { "responses": {} } } } });
    let out = merged(vec![input("a", "", a), input("b", "", b)]).unwrap();

    let names = |method: &str| -> Vec<Value> {
      let parameters = out
        .spec
        .pointer(&format!("/paths/~1pets~1{{id}}/{method}/parameters"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
      parameters.iter().map(|p| p["name"].clone()).collect()
    };

    // Neither source's path-level parameters leak onto the other's operations.
    assert_eq!(names("get"), [json!("id")]);
    assert_eq!(names("delete"), [json!("pet")]);
    assert!(out.spec.pointer("/paths/~1pets~1{id}/parameters").is_none());
  }

  #[test]
  fn a_shared_operation_id_is_a_conflict_a_prefix_resolves() {
    let error = merged(vec![input("orders", "", orders()), input("billing", "", billing())]).err().unwrap().to_string();
    assert!(error.starts_with("operationId list is used by more than one operation: orders and billing"), "{error}");
    assert!(error.ends_with("Give one of the sources a prefix"), "{error}");

    let out = merged(vec![input("orders", "", orders()), input("billing", "Billing", billing())]).unwrap();
    let ids: Vec<_> = out.operations.iter().filter_map(|o| o.operation_id.clone()).collect();
    assert_eq!(ids, ["list", "Billing_list"]);
    assert_eq!(out.operations[1].tags, ["Billing Invoices"]);
  }

  #[test]
  fn clashing_components_are_a_conflict_unless_identical_or_prefixed() {
    let plain_a = spec("A", "1", json!({}), json!({ "schemas": { "Page": { "type": "object" } } }));
    let same = spec("B", "1", json!({}), json!({ "schemas": { "Page": { "type": "object" } } }));
    assert!(merged(vec![input("a", "", plain_a), input("b", "", same)]).is_ok());

    let error = merged(vec![
      input("orders", "", orders()),
      input("billing", "", spec("B", "1", json!({ "/x": { "get": { "responses": {} } } }), billing()["components"].clone())),
    ])
    .err()
    .unwrap()
    .to_string();
    assert!(error.starts_with("schemas/Page is defined differently: orders and billing"), "{error}");

    // With the prefix both survive, and the references of the prefixed source follow its component.
    let out = merged(vec![input("orders", "", orders()), input("billing", "Billing", billing())]).unwrap();
    assert!(out.spec.pointer("/components/schemas/Page").is_some() && out.spec.pointer("/components/schemas/Billing_Page").is_some());
    assert_eq!(
      out.spec.pointer("/paths/~1invoices/get/responses/200/content/application~1json/schema/$ref"),
      Some(&json!("#/components/schemas/Billing_Page"))
    );
    assert_eq!(
      out.spec.pointer("/paths/~1orders/get/responses/200/content/application~1json/schema/$ref"),
      Some(&json!("#/components/schemas/Page"))
    );
  }

  #[test]
  fn a_prefix_gives_untagged_operations_its_name_and_security_follows_renamed_schemes() {
    let secured = json!({
      "openapi": "3.0.3", "info": { "title": "S", "version": "1" },
      "security": [{ "Key": [] }],
      "paths": { "/s": { "get": { "responses": {} } } },
      "components": { "securitySchemes": { "Key": { "type": "apiKey", "in": "header", "name": "k" } } }
    });
    let other = spec("O", "1", json!({ "/o": { "get": { "responses": {} } } }), json!({}));
    let out = merged(vec![input("other", "", other), input("secured", "S", secured)]).unwrap();

    assert_eq!(out.operations[1].tags, ["S"]);
    assert_eq!(out.spec.pointer("/paths/~1s/get/security/0/S_Key"), Some(&json!([])));
    assert!(out.spec.pointer("/components/securitySchemes/S_Key").is_some());
    // The other source's operations do not inherit it, and the document has no top-level requirement of its own.
    assert!(out.spec.pointer("/paths/~1o/get/security").is_none() && out.spec.get("security").is_none());
  }

  #[test]
  fn overrides_servers_tags_and_version_follow_the_options() {
    let a = json!({ "openapi": "3.0.3", "info": { "title": "A", "version": "1" }, "servers": [{ "url": "https://a.example.com" }], "tags": [{ "name": "X", "description": "first" }], "paths": {} });
    let b = json!({ "openapi": "3.1.0", "info": { "title": "B", "version": "2" }, "tags": [{ "name": "X", "description": "second" }, { "name": "Y" }], "paths": {} });

    let out = merged(vec![input("a", "", a.clone()), input("b", "", b.clone())]).unwrap();
    assert_eq!(out.spec.get("openapi"), Some(&json!("3.1.0")));
    assert_eq!(out.spec.pointer("/servers/0/url"), Some(&json!("https://a.example.com")));
    assert_eq!(out.spec.get("tags"), Some(&json!([{ "name": "X", "description": "first" }, { "name": "Y" }])));

    let options = MergeOptions {
      title: Some("Everything".into()),
      version: Some("9".into()),
      description: Some("All of it".into()),
      servers: vec!["https://api.example.com".into()],
    };
    let out = merge(vec![input("a", "", a), input("b", "", b)], &options).unwrap();
    assert_eq!(out.spec.get("info"), Some(&json!({ "title": "Everything", "version": "9", "description": "All of it" })));
    assert_eq!(out.spec.get("servers"), Some(&json!([{ "url": "https://api.example.com" }])));
  }

  #[test]
  fn the_result_does_not_depend_on_anything_but_the_inputs() {
    let run = || {
      let out = merged(vec![
        input("orders", "", orders()),
        input("billing", "Billing", {
          let mut b = billing();
          b["paths"]["/invoices"]["get"]["operationId"] = json!("listInvoices");
          b
        }),
      ])
      .unwrap();

      serde_json::to_string(&out.spec).unwrap()
    };

    assert_eq!(run(), run());
  }

  #[test]
  fn no_sources_is_refused() {
    assert!(matches!(merged(vec![]), Err(ApiSpecError::Invalid(_))));
  }
}
