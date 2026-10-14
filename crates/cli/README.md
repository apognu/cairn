# cli

The `cairn` binary: `dev` (local, no login), `serve` (a deployment: OIDC, one port) and `openapi`; migrate, export, backup, restore and admin come later.
Not obvious: it holds no logic, it parses arguments and calls `cairn-server`.
