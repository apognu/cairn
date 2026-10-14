use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// One macro for all ID newtypes: they are identical and must stay so, so a mix-up between a page and a release is a type error.
macro_rules! id_type {
  ($(#[$doc:meta])* $name:ident) => {
    $(#[$doc])*
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
    #[serde(transparent)]
    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema), schema(value_type = String))]
    pub struct $name(Uuid);

    impl $name {
      /// A new time-ordered (UUIDv7) ID.
      pub fn new() -> Self {
        Self(Uuid::now_v7())
      }

      pub fn as_uuid(&self) -> Uuid {
        self.0
      }
    }

    impl Default for $name {
      fn default() -> Self {
        Self::new()
      }
    }

    impl From<Uuid> for $name {
      fn from(uuid: Uuid) -> Self {
        Self(uuid)
      }
    }

    impl fmt::Display for $name {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
      }
    }

    impl FromStr for $name {
      type Err = uuid::Error;

      fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
      }
    }
  };
}

id_type!(
  /// Stable identity of a page; URLs derive from the tree, never from this.
  PageId
);
id_type!(
  /// An immutable snapshot of a page's document.
  RevisionId
);
id_type!(
  /// An immutable published state of the whole site.
  ReleaseId
);
id_type!(
  /// A user, including the built-in local one.
  UserId
);
id_type!(
  /// An uploaded file.
  AssetId
);
id_type!(
  /// What a page's lock holder sends with each save and renewal; a new one is made whenever the lock changes hands.
  LockToken
);

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn round_trips_as_a_plain_string() {
    let id = PageId::new();
    let json = serde_json::to_string(&id).unwrap();

    assert_eq!(json, format!("\"{id}\""));
    assert_eq!(serde_json::from_str::<PageId>(&json).unwrap(), id);
    assert_eq!(id.to_string().parse::<PageId>().unwrap(), id);
  }
}
