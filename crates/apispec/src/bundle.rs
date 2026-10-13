use std::collections::{BTreeMap, HashMap, HashSet};

use serde_json::{Map, Value};
use url::Url;

use crate::{
  ApiSpecError,
  pointer::{resolve, split_ref},
};

/// More files than this means a spec that points all over the internet.
const MAX_DOCS: usize = 200;
/// Components pulled in from other files, in all.
const MAX_COMPONENTS: usize = 5000;
/// Path items that are references to references to references.
const MAX_ALIAS_DEPTH: usize = 20;

/// Where files a spec points to come from. The publisher fetches them over HTTPS; tests use a map.
pub trait Fetch {
  fn fetch(&self, url: &str) -> impl Future<Output = Result<Vec<u8>, String>> + Send;
}

/// Makes a spec self-contained: every `$ref` to another file is replaced by a reference to a component of the document, and the
/// component is the referenced content, named after its file or pointer. References inside the other files are followed the same way,
/// so cycles end where a component is already being built.
pub async fn bundle(mut root: Value, base_url: &str, fetch: &impl Fetch) -> Result<Value, ApiSpecError> {
  let base = Url::parse(base_url).map_err(|_| ApiSpecError::Invalid(format!("{base_url} is not a valid address")))?;
  let mut bundler = Bundler::new(&root, &base);

  bundler.inline_path_items(&mut root, fetch).await?;
  bundler.rewrite(&mut root, &base, true, "schemas", false)?;
  bundler.drain(fetch).await?;
  bundler.finish(&mut root);

  Ok(root)
}

struct Job {
  url: String,
  pointer: String,
  section: String,
  name: String,
}

struct Bundler {
  root: String,
  docs: HashMap<String, Value>,
  /// (file, pointer) to the `$ref` that now stands for it.
  refs: HashMap<(String, String), String>,
  /// (section, name) of every component, the document's own included.
  taken: HashSet<(String, String)>,
  queue: Vec<Job>,
  done: BTreeMap<String, BTreeMap<String, Value>>,
  count: usize,
}

impl Bundler {
  fn new(root: &Value, base: &Url) -> Self {
    let mut taken = HashSet::new();
    if let Some(Value::Object(components)) = root.get("components") {
      for (section, entries) in components {
        if let Value::Object(entries) = entries {
          taken.extend(entries.keys().map(|name| (section.clone(), name.clone())));
        }
      }
    }

    Bundler {
      root: without_fragment(base),
      docs: HashMap::new(),
      refs: HashMap::new(),
      taken,
      queue: Vec::new(),
      done: BTreeMap::new(),
      count: 0,
    }
  }

  /// A path item that is a reference is replaced by the file's content: components are no place for them before OpenAPI 3.1.
  async fn inline_path_items(&mut self, root: &mut Value, fetch: &impl Fetch) -> Result<(), ApiSpecError> {
    let base = Url::parse(&self.root).expect("the root address was parsed before");

    for section in ["paths", "webhooks"] {
      let Some(Value::Object(items)) = root.get_mut(section) else { continue };

      for item in items.values_mut() {
        let mut at = base.clone();

        for _ in 0..MAX_ALIAS_DEPTH {
          let Some(reference) = ref_of(item) else { break };
          let (file, pointer) = split_ref(&reference);
          let target = join(&at, file)?;
          let document = self.document(&target, fetch).await?;
          let content = resolve(&document, &pointer)
            .cloned()
            .ok_or_else(|| ApiSpecError::Invalid(format!("{reference} does not exist in {target}")))?;

          *item = content;
          at = target;
        }
        if ref_of(item).is_some() {
          return Err(ApiSpecError::Limit("a path item refers to another that refers back".into()));
        }

        // The content now sits in this document, but its own references mean what they meant in the file it came from.
        if at.as_str() != base.as_str() {
          self.rewrite(item, &at, false, "schemas", false)?;
        }
      }
    }

    Ok(())
  }

  /// Replaces the references of `value`. In the root document the ones to itself (`#/…`) stay; in any other file every reference is to
  /// something that has to come along.
  fn rewrite(&mut self, value: &mut Value, base: &Url, is_root: bool, hint: &str, keyed: bool) -> Result<(), ApiSpecError> {
    match value {
      Value::Array(items) => {
        for item in items {
          self.rewrite(item, base, is_root, "schemas", false)?;
        }
      }
      Value::Object(map) => {
        if let Some(Value::String(reference)) = map.get("$ref") {
          let reference = reference.clone();
          if let Some(new) = self.reference(&reference, base, is_root, hint)? {
            map.insert("$ref".into(), Value::String(new));
          }
          return Ok(());
        }

        for (key, child) in map.iter_mut() {
          // The keys of a map of schemas, responses, … are names the author chose, never keywords.
          if keyed {
            self.rewrite(child, base, is_root, hint, false)?;
            continue;
          }

          match key.as_str() {
            "example" | "default" | "enum" | "const" => {}
            "properties" | "patternProperties" | "$defs" | "definitions" | "schemas" | "dependentSchemas" | "content" => self.rewrite(child, base, is_root, "schemas", true)?,
            "paths" | "webhooks" => self.rewrite(child, base, is_root, "pathItems", true)?,
            "requestBody" => self.rewrite(child, base, is_root, "requestBodies", false)?,
            _ => match section_of(key) {
              Some(section) => match child {
                Value::Array(items) => {
                  for item in items {
                    self.rewrite(item, base, is_root, section, false)?;
                  }
                }
                // A reference where a map of these was expected is not one of them (`properties: { examples: { $ref } }`).
                Value::Object(m) if m.get("$ref").is_some_and(Value::is_string) => self.rewrite(child, base, is_root, "schemas", false)?,
                _ => self.rewrite(child, base, is_root, section, true)?,
              },
              None => self.rewrite(child, base, is_root, "schemas", false)?,
            },
          }
        }
      }
      _ => {}
    }

    Ok(())
  }

  /// What a `$ref` becomes: `None` when it stays as it is.
  fn reference(&mut self, reference: &str, base: &Url, is_root: bool, hint: &str) -> Result<Option<String>, ApiSpecError> {
    if is_root && reference.starts_with('#') {
      return Ok(None);
    }

    let (file, pointer) = split_ref(reference);
    if !pointer.is_empty() && !pointer.starts_with('/') {
      return Err(ApiSpecError::Invalid(format!(
        "{reference} is not a reference this tool can follow (use a JSON pointer such as #/components/schemas/Name)"
      )));
    }
    let target = without_fragment(&join(base, file)?);

    // A file that points back at the document being built.
    if target == self.root {
      return Ok(Some(format!("#{}", reference.split_once('#').map_or("", |(_, f)| f))));
    }

    let key = (target.clone(), pointer.clone());
    if let Some(known) = self.refs.get(&key) {
      return Ok(Some(known.clone()));
    }

    self.count += 1;
    if self.count > MAX_COMPONENTS {
      return Err(ApiSpecError::Limit(format!("the spec pulls in more than {MAX_COMPONENTS} components from other files")));
    }

    let (section, name) = self.name_for(&target, &pointer, hint);
    let new = format!("#/components/{section}/{name}");
    self.refs.insert(key, new.clone());
    self.queue.push(Job { url: target, pointer, section, name });

    Ok(Some(new))
  }

  fn name_for(&mut self, url: &str, pointer: &str, hint: &str) -> (String, String) {
    let parts: Vec<&str> = pointer.split('/').skip(1).collect();
    let (section, wanted) = match parts.as_slice() {
      ["components", section, name] => (section.to_string(), unescape(name)),
      _ => {
        let last = parts.last().map(|p| unescape(p));
        let file = Url::parse(url).ok().and_then(|u| u.path_segments().and_then(|mut s| s.next_back().map(str::to_string)));
        let stem = file.map(|f| f.rsplit_once('.').map_or(f.clone(), |(stem, _)| stem.to_string()));
        (hint.to_string(), last.or(stem).unwrap_or_default())
      }
    };

    let clean: String = wanted.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') { c } else { '_' }).collect();
    let clean = if clean.is_empty() { "Ref".to_string() } else { clean };

    let mut name = clean.clone();
    let mut n = 2;
    while self.taken.contains(&(section.clone(), name.clone())) {
      name = format!("{clean}_{n}");
      n += 1;
    }
    self.taken.insert((section.clone(), name.clone()));

    (section, name)
  }

  /// Fetches the files the references lead to, which may lead to more.
  async fn drain(&mut self, fetch: &impl Fetch) -> Result<(), ApiSpecError> {
    while let Some(job) = self.queue.pop() {
      let target = Url::parse(&job.url).expect("addresses in the queue were parsed");
      let document = self.document(&target, fetch).await?;
      let mut content = resolve(&document, &job.pointer)
        .cloned()
        .ok_or_else(|| ApiSpecError::Invalid(format!("#{} does not exist in {}", job.pointer, job.url)))?;

      self.rewrite(&mut content, &target, false, &job.section, false)?;
      self.done.entry(job.section).or_default().insert(job.name, content);
    }

    Ok(())
  }

  async fn document(&mut self, url: &Url, fetch: &impl Fetch) -> Result<Value, ApiSpecError> {
    let key = without_fragment(url);
    if let Some(known) = self.docs.get(&key) {
      return Ok(known.clone());
    }
    if self.docs.len() >= MAX_DOCS {
      return Err(ApiSpecError::Limit(format!("the spec points to more than {MAX_DOCS} files")));
    }

    let bytes = fetch.fetch(&key).await.map_err(|message| ApiSpecError::Fetch { url: key.clone(), message })?;
    let document = crate::parse(&bytes).map_err(|e| ApiSpecError::Invalid(format!("{key}: {e}")))?;
    self.docs.insert(key, document.clone());

    Ok(document)
  }

  fn finish(self, root: &mut Value) {
    if self.done.is_empty() {
      return;
    }
    let Some(root) = root.as_object_mut() else { return };
    let components = root.entry("components").or_insert_with(|| Value::Object(Map::new()));
    let Some(components) = components.as_object_mut() else { return };

    for (section, entries) in self.done {
      let target = components.entry(section).or_insert_with(|| Value::Object(Map::new()));
      if let Some(target) = target.as_object_mut() {
        target.extend(entries);
      }
    }
  }
}

/// The reference a path item is, when it is nothing else.
fn ref_of(value: &Value) -> Option<String> {
  value.get("$ref")?.as_str().map(str::to_string)
}

fn section_of(key: &str) -> Option<&'static str> {
  Some(match key {
    "parameters" => "parameters",
    "responses" => "responses",
    "headers" => "headers",
    "examples" => "examples",
    "callbacks" => "callbacks",
    "links" => "links",
    "securitySchemes" => "securitySchemes",
    "requestBodies" => "requestBodies",
    _ => return None,
  })
}

fn join(base: &Url, file: &str) -> Result<Url, ApiSpecError> {
  if file.is_empty() {
    return Ok(base.clone());
  }

  let joined = base.join(file).map_err(|_| ApiSpecError::Invalid(format!("{file} is not a valid reference from {base}")))?;
  if !matches!(joined.scheme(), "http" | "https") {
    return Err(ApiSpecError::Invalid(format!("{file} is not an http or https reference")));
  }

  Ok(joined)
}

fn without_fragment(url: &Url) -> String {
  let mut url = url.clone();
  url.set_fragment(None);

  url.to_string()
}

fn unescape(part: &str) -> String {
  part.replace("~1", "/").replace("~0", "~")
}
