use rust_embed::Embed;

/// The bundles built by `bun run build`: `ssr.js` for V8, `client.js` for hydration and `site.css`, the shared site stylesheet.
#[derive(Embed)]
#[folder = "../../packages/renderer/dist"]
#[include = "*.js"]
#[include = "*.css"]
pub struct Assets;
