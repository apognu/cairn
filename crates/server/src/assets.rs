use rust_embed::Embed;

/// The two bundles built by `bun run build`: `ssr.js` for V8 and `client.js` for hydration.
#[derive(Embed)]
#[folder = "../../packages/renderer/dist"]
#[include = "*.js"]
pub struct Assets;
