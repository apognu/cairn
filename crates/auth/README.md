# auth

Identifies the user behind a request via the `CurrentUser` extractor.
Not obvious: until M5 it always yields the fixed local user; handlers must take `CurrentUser` for every write so OIDC replaces it in one place.
