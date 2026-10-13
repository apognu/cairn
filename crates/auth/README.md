# auth

The OIDC login (`oidc.rs`, `routes.rs`) and the `CurrentUser` extractor every writing handler takes.
Not obvious: `Auth::local()` (used by `dev`) makes every request the fixed local user, which is not a row in `users`; the provider's `(issuer, subject)` is what identifies a real user, never the email.
ID tokens are validated by `openidconnect`; the groups claim is read from the payload afterwards, because its name is configurable.
