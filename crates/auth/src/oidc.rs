//! The OpenID Connect login: authorization code with PKCE, and validation of the ID token.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use openidconnect::{
  AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointMaybeSet, EndpointNotSet, EndpointSet, IssuerUrl, Nonce, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope,
  TokenResponse,
  core::{CoreClient, CoreIdToken, CoreIdTokenVerifier, CoreProviderMetadata, CoreResponseType},
  reqwest,
};

use crate::AuthError;

/// A client built from discovery: the authorization endpoint is known, the token and userinfo endpoints are checked when used.
type Client = CoreClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointMaybeSet, EndpointMaybeSet>;

/// How the deployment's identity provider is reached and what to read from its tokens.
#[derive(Debug, Clone)]
pub struct OidcConfig {
  pub issuer: String,
  pub client_id: String,
  pub client_secret: cairn_core::Secret,
  /// Scopes asked besides `openid`.
  pub scopes: Vec<String>,
  /// The ID token claim holding the user's groups.
  pub groups_claim: String,
}

/// Who the provider says signed in. The pair `(issuer, subject)` identifies a user for good; the rest follows the provider at each login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
  pub issuer: String,
  pub subject: String,
  pub email: String,
  pub email_verified: bool,
  pub name: String,
  pub groups: Vec<String>,
}

/// Where to send the browser, and what the callback needs to remember.
pub struct Authorization {
  pub url: String,
  pub state: String,
  pub nonce: String,
  pub verifier: String,
}

pub struct Oidc {
  config: OidcConfig,
  metadata: CoreProviderMetadata,
  http: reqwest::Client,
  redirect: RedirectUrl,
}

impl Oidc {
  /// Reads the provider's discovery document. `redirect` is where the provider sends the browser back, `<PUBLIC_URL>/auth/callback`.
  pub async fn discover(config: OidcConfig, redirect: &str) -> Result<Self, AuthError> {
    // Following redirects opens the client up to SSRF: the provider's endpoints do not redirect.
    let http = reqwest::ClientBuilder::new()
      .redirect(reqwest::redirect::Policy::none())
      .timeout(std::time::Duration::from_secs(10))
      .build()
      .map_err(|e| AuthError::Other(e.into()))?;
    let issuer = IssuerUrl::new(config.issuer.clone()).map_err(|e| AuthError::Config(format!("OIDC_ISSUER is not a URL: {e}")))?;
    let metadata = CoreProviderMetadata::discover_async(issuer, &http)
      .await
      .map_err(|e| AuthError::Config(format!("cannot read the OIDC configuration of {}: {e}", config.issuer)))?;
    let redirect = RedirectUrl::new(redirect.to_string()).map_err(|e| AuthError::Config(format!("the callback address is not a URL: {e}")))?;

    Ok(Self { config, metadata, http, redirect })
  }

  /// A provider that is never contacted, for tests of what happens after a login.
  #[cfg(any(test, feature = "testing"))]
  pub fn unreachable() -> Self {
    use openidconnect::{AuthUrl, EmptyAdditionalProviderMetadata, JsonWebKeySetUrl, ResponseTypes, core::CoreSubjectIdentifierType};

    let metadata = CoreProviderMetadata::new(
      IssuerUrl::new("https://idp.example".into()).unwrap(),
      AuthUrl::new("https://idp.example/auth".into()).unwrap(),
      JsonWebKeySetUrl::new("https://idp.example/keys".into()).unwrap(),
      vec![ResponseTypes::new(vec![CoreResponseType::Code])],
      vec![CoreSubjectIdentifierType::Public],
      vec![openidconnect::core::CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256],
      EmptyAdditionalProviderMetadata {},
    );

    Self {
      config: OidcConfig {
        issuer: "https://idp.example".into(),
        client_id: "editor".into(),
        client_secret: Default::default(),
        scopes: vec![],
        groups_claim: "groups".into(),
      },
      metadata,
      http: reqwest::Client::new(),
      redirect: RedirectUrl::new("https://docs.example/auth/callback".into()).unwrap(),
    }
  }

  fn client(&self) -> Client {
    CoreClient::from_provider_metadata(
      self.metadata.clone(),
      ClientId::new(self.config.client_id.clone()),
      Some(ClientSecret::new(self.config.client_secret.expose().to_string())),
    )
    .set_redirect_uri(self.redirect.clone())
  }

  /// The address to send the browser to, with a fresh state, nonce and PKCE challenge.
  pub fn authorize(&self) -> Authorization {
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
    let client = self.client();
    let mut request = client
      .authorize_url(AuthenticationFlow::<CoreResponseType>::AuthorizationCode, CsrfToken::new_random, Nonce::new_random)
      .set_pkce_challenge(challenge);

    for scope in ["profile", "email"].into_iter().map(String::from).chain(self.config.scopes.iter().cloned()) {
      request = request.add_scope(Scope::new(scope));
    }
    let (url, state, nonce) = request.url();

    Authorization {
      url: url.to_string(),
      state: state.secret().clone(),
      nonce: nonce.secret().clone(),
      verifier: verifier.secret().clone(),
    }
  }

  /// Trades the code for tokens and validates the ID token: signature, issuer, audience, expiry and the nonce of this login.
  pub async fn complete(&self, code: String, verifier: String, nonce: String) -> Result<Identity, AuthError> {
    let client = self.client();
    let tokens = client
      .exchange_code(AuthorizationCode::new(code))
      .map_err(|e| AuthError::Config(format!("the provider has no token endpoint: {e}")))?
      .set_pkce_verifier(PkceCodeVerifier::new(verifier))
      .request_async(&self.http)
      .await
      .map_err(|e| AuthError::Login(format!("the provider refused the code: {e}")))?;
    let id_token = tokens.id_token().ok_or_else(|| AuthError::Login("the provider sent no ID token".into()))?;

    identity_from_token(id_token, &client.id_token_verifier(), &nonce, &self.config.groups_claim)
  }
}

fn identity_from_token(id_token: &CoreIdToken, verifier: &CoreIdTokenVerifier, nonce: &str, groups_claim: &str) -> Result<Identity, AuthError> {
  let claims = id_token
    .claims(verifier, &Nonce::new(nonce.to_string()))
    .map_err(|e| AuthError::Login(format!("the ID token is not valid: {e}")))?;

  let email = claims.email().map(|e| e.as_str().to_string()).unwrap_or_default();
  let name = claims
    .name()
    .and_then(|n| n.get(None))
    .map(|n| n.as_str().to_string())
    .or_else(|| claims.preferred_username().map(|n| n.as_str().to_string()))
    .filter(|n| !n.is_empty())
    .unwrap_or_else(|| email.clone());

  Ok(Identity {
    issuer: claims.issuer().as_str().to_string(),
    subject: claims.subject().as_str().to_string(),
    // A provider that does not say may let people type any address (Azure AD's `email` is one): unverified until it says otherwise.
    email_verified: claims.email_verified().unwrap_or(false),
    email,
    name,
    // The library validated the token as a whole, so its payload can be read for the claims it does not model.
    groups: groups_from_payload(&id_token.to_string(), groups_claim),
  })
}

/// The groups in a JWT's payload: a list of strings, or one string. Anything else counts as no groups.
fn groups_from_payload(jwt: &str, claim: &str) -> Vec<String> {
  let payload = jwt
    .split('.')
    .nth(1)
    .and_then(|p| URL_SAFE_NO_PAD.decode(p).ok())
    .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());

  match payload.as_ref().and_then(|p| p.get(claim)) {
    Some(serde_json::Value::Array(items)) => items.iter().filter_map(|i| i.as_str().map(String::from)).collect(),
    Some(serde_json::Value::String(one)) => vec![one.clone()],
    _ => vec![],
  }
}

#[cfg(test)]
mod tests {
  use openidconnect::{
    Audience, EmptyAdditionalClaims, EndUserEmail, JsonWebKeyId, StandardClaims, SubjectIdentifier,
    core::{CoreIdTokenClaims, CoreJsonWebKeySet, CoreJwsSigningAlgorithm, CoreRsaPrivateSigningKey},
  };

  use super::*;

  // A key made only for these tests; it signs nothing real.
  const TEST_KEY: &str = include_str!("test_key.pem");
  const ISSUER: &str = "https://idp.example";

  fn token(audience: &str, nonce: &str) -> (CoreIdToken, CoreIdTokenVerifier<'static>) {
    let key = CoreRsaPrivateSigningKey::from_pem(TEST_KEY, Some(JsonWebKeyId::new("k1".into()))).unwrap();
    let issuer = IssuerUrl::new(ISSUER.into()).unwrap();
    let claims = CoreIdTokenClaims::new(
      issuer.clone(),
      vec![Audience::new(audience.into())],
      chrono::Utc::now() + chrono::Duration::minutes(5),
      chrono::Utc::now(),
      StandardClaims::new(SubjectIdentifier::new("alice-1".into())).set_email(Some(EndUserEmail::new("Alice@Example.com".into()))),
      EmptyAdditionalClaims {},
    )
    .set_nonce(Some(Nonce::new(nonce.into())));

    let id_token = CoreIdToken::new(claims, &key, CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256, None, None).unwrap();
    let jwks = CoreJsonWebKeySet::new(vec![openidconnect::PrivateSigningKey::as_verification_key(&key)]);

    (id_token, CoreIdTokenVerifier::new_public_client(ClientId::new("editor".into()), issuer, jwks))
  }

  #[test]
  fn a_valid_token_gives_the_identity() {
    let (id_token, verifier) = token("editor", "n1");

    let identity = identity_from_token(&id_token, &verifier, "n1", "groups").unwrap();

    assert_eq!((identity.issuer.as_str(), identity.subject.as_str()), (ISSUER, "alice-1"));
    assert_eq!(identity.email, "Alice@Example.com");
  }

  // (audience of the token, nonce the login expects): only the first pair is ours.
  #[test]
  fn a_token_for_another_audience_or_login_is_refused() {
    for (audience, nonce) in [("someone-else", "n1"), ("editor", "another-login")] {
      let (id_token, verifier) = token(audience, "n1");

      let error = identity_from_token(&id_token, &verifier, nonce, "groups").expect_err("must refuse");

      assert!(matches!(error, AuthError::Login(_)), "{audience} {nonce}");
    }
  }

  #[test]
  fn groups_are_read_from_the_named_claim() {
    let payload = |json: &str| format!("h.{}.s", URL_SAFE_NO_PAD.encode(json));

    for (json, claim, expected) in [
      (r#"{"groups":["docs","ops"]}"#, "groups", vec!["docs", "ops"]),
      (r#"{"roles":"docs"}"#, "roles", vec!["docs"]),
      (r#"{"groups":[1,"a"]}"#, "groups", vec!["a"]),
      (r#"{"groups":"x"}"#, "other", vec![]),
      ("not json", "groups", vec![]),
    ] {
      assert_eq!(groups_from_payload(&payload(json), claim), expected, "{json}");
    }
  }
}
