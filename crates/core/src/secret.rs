use std::fmt;

/// A credential from the configuration (a client secret, an API token). Its `Debug` output is redacted, so a struct holding one can be
/// logged or end up in a panic message without leaking it.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
  /// The value itself, for the one place that sends it.
  pub fn expose(&self) -> &str {
    &self.0
  }
}

impl fmt::Debug for Secret {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("Secret(***)")
  }
}

impl From<String> for Secret {
  fn from(value: String) -> Self {
    Secret(value)
  }
}

impl From<&str> for Secret {
  fn from(value: &str) -> Self {
    Secret(value.to_string())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn debug_output_hides_the_value() {
    let secret = Secret::from("hunter2");

    assert_eq!(format!("{secret:?}"), "Secret(***)");
    assert_eq!(secret.expose(), "hunter2");
  }
}
