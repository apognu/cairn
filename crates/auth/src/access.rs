//! Who may sign in. What they may do once in is their role, stored with the user (`users.role`), not decided here.

use crate::Identity;

/// The deployment's allow-list. A user must match a group or an email domain, unless the deployment lets everyone the provider
/// authenticates in (`allow_any`, an explicit choice because it hands the editor to the whole provider).
#[derive(Debug, Clone, Default)]
pub struct AccessRules {
  /// Provider groups; one match is enough.
  pub groups: Vec<String>,
  /// Lowercased email domains, without the `@`.
  pub email_domains: Vec<String>,
  pub allow_any: bool,
}

impl AccessRules {
  /// Whether nothing was configured, which no deployment may run with.
  pub fn is_empty(&self) -> bool {
    self.groups.is_empty() && self.email_domains.is_empty() && !self.allow_any
  }

  pub fn allows(&self, identity: &Identity) -> bool {
    if self.allow_any {
      return true;
    }

    let in_group = identity.groups.iter().any(|g| self.groups.contains(g));
    // An address the provider did not verify proves nothing about its domain.
    let in_domain = identity.email_verified
      && identity
        .email
        .rsplit_once('@')
        .is_some_and(|(_, domain)| self.email_domains.iter().any(|allowed| allowed.eq_ignore_ascii_case(domain)));

    in_group || in_domain
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn identity(email: &str, verified: bool, groups: &[&str]) -> Identity {
    Identity {
      issuer: "https://idp.example".into(),
      subject: "s".into(),
      email: email.into(),
      email_verified: verified,
      name: "A".into(),
      groups: groups.iter().map(|g| g.to_string()).collect(),
    }
  }

  #[test]
  fn a_user_must_match_a_group_or_a_domain() {
    let rules = AccessRules {
      groups: vec!["docs".into()],
      email_domains: vec!["example.com".into()],
      allow_any: false,
    };

    // (identity, allowed)
    for (who, allowed) in [
      (identity("a@other.org", true, &["docs"]), true),
      (identity("a@example.com", true, &[]), true),
      (identity("a@EXAMPLE.com", true, &[]), true),
      (identity("a@example.com", false, &[]), false),
      (identity("a@other.org", true, &["ops"]), false),
      (identity("a@other.org", true, &[]), false),
      (identity("", true, &[]), false),
      (identity("a@evil.example.com.attacker.io", true, &[]), false),
    ] {
      assert_eq!(rules.allows(&who), allowed, "{who:?}");
    }
  }

  #[test]
  fn allowing_anyone_is_explicit() {
    assert!(AccessRules::default().is_empty());
    assert!(!AccessRules::default().allows(&identity("a@example.com", true, &[])));
    assert!(
      AccessRules {
        allow_any: true,
        ..Default::default()
      }
      .allows(&identity("a@other.org", false, &[]))
    );
  }
}
