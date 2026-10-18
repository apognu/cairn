//! The pages a person sees during sign-in: the choice of provider, a failed sign-in, signing out. They are served before the editor
//! loads, so they carry their own styles: the editor's tokens and its splash screen's look, in both modes.

/// The drawing in the round badge at the top of the card.
#[derive(Clone, Copy)]
pub(crate) enum Badge {
  SignIn,
  SignedOut,
  Failed,
}

impl Badge {
  fn path(self) -> &'static str {
    match self {
      Badge::SignIn => "M7 11h10a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2v-6a2 2 0 0 1 2-2zM8 11V8a4 4 0 0 1 8 0v3",
      Badge::SignedOut => "M15 4h3a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2h-3M10 8l-4 4 4 4M6 12h10",
      Badge::Failed => "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM12 7.5V13M12 16.5v.5",
    }
  }
}

pub(crate) struct Action<'a> {
  pub label: &'a str,
  pub href: String,
  pub primary: bool,
}

const LOGO: &str = "M3 19.5C3 17.6 7 16.5 11.5 16.5S20 17.6 20 19.5c0 1-3.5 1.5-8.5 1.5S3 20.5 3 19.5zM7 14.5c0-1.4 2.7-2.5 6-2.5s6 1 6 2.5c0 1.2-2.7 2-6 2s-6-.8-6-2zM5.5 10.2C5.5 9 7.5 8 10 8s4.5 1 4.5 2.2c0 1.1-2 1.8-4.5 1.8s-4.5-.7-4.5-1.8zM9.2 6.2c0-1.3 1.1-2.2 2.5-2.2s2.5.9 2.5 2.2c0 .9-1.1 1.6-2.5 1.6S9.2 7.1 9.2 6.2z";

// The values of `packages/renderer/src/site.css` and the editor's button tokens; a saved theme does not apply before sign-in.
const STYLE: &str = r#"
:root{--bg:#fff;--border:#e3e5e0;--text:#16181a;--muted:#5b6168;--accent:#0b6e4f;--danger:#b3261e;--button-bg:var(--bg);--button-hover:#eef0ec;--button-border:#d5d8d2;--button-shadow:0 1px 2px rgb(16 20 22/7%);--card-shadow:0 1px 2px rgb(16 20 22/5%),0 8px 24px rgb(16 20 22/6%);color-scheme:light}
@media (prefers-color-scheme:dark){:root{--bg:#121416;--border:#2a2e32;--text:#eceeef;--muted:#9aa1a8;--accent:color-mix(in srgb,#0b6e4f 55%,white);--danger:#f2b8b5;--button-bg:color-mix(in srgb,var(--text) 6%,var(--bg));--button-hover:color-mix(in srgb,var(--text) 11%,var(--bg));--button-border:color-mix(in srgb,var(--text) 15%,var(--bg));--button-shadow:inset 0 1px 0 rgb(255 255 255/5%),0 1px 2px rgb(0 0 0/35%);--card-shadow:0 8px 24px rgb(0 0 0/30%);color-scheme:dark}}
*{box-sizing:border-box}
body{margin:0;min-height:100dvh;display:grid;place-items:center;padding:24px 16px;background:var(--bg);color:var(--text);font:14px/1.5 "Hanken Grotesk",system-ui,-apple-system,"Segoe UI",sans-serif}
main{display:flex;flex-direction:column;align-items:center;gap:20px;width:100%;max-width:380px}
.logo{color:var(--muted)}
.card{width:100%;padding:28px;border:1px solid var(--border);border-radius:14px;background:var(--bg);box-shadow:var(--card-shadow);text-align:center}
.badge{display:inline-grid;place-items:center;width:44px;height:44px;margin-bottom:14px;border-radius:50%;color:var(--accent);background:color-mix(in srgb,var(--accent) 11%,var(--bg))}
.badge.failed{color:var(--danger);background:color-mix(in srgb,var(--danger) 11%,var(--bg))}
h1{margin:0 0 6px;font-size:17px;font-weight:600;line-height:1.3}
p{margin:0;color:var(--muted);overflow-wrap:anywhere}
.actions{display:flex;flex-direction:column;gap:8px;margin-top:22px}
.actions a{display:flex;align-items:center;justify-content:center;height:34px;padding:0 14px;border-radius:7px;font-size:13.5px;font-weight:600;text-decoration:none;transition:opacity .1s,background-color .1s}
.primary{background:var(--text);color:var(--bg);box-shadow:var(--button-shadow)}
.primary:hover{opacity:.88}
.secondary{border:1px solid var(--button-border);background:var(--button-bg);color:var(--text);box-shadow:var(--button-shadow)}
.secondary:hover{background:var(--button-hover)}
a:focus-visible{outline:2px solid var(--accent);outline-offset:2px}
"#;

/// A whole page: the logo over a card with the badge, a heading, a message and the actions, the first of which takes the focus.
pub(crate) fn page(title: &str, badge: Badge, heading: &str, message: &str, actions: &[Action]) -> String {
  let svg = |size: u32, path: &str| {
    format!("<svg width={size} height={size} viewBox=\"0 0 24 24\" fill=none stroke=currentColor stroke-width=2 stroke-linecap=round stroke-linejoin=round aria-hidden=true><path d=\"{path}\"/></svg>")
  };
  let links: String = actions
    .iter()
    .enumerate()
    .map(|(i, a)| {
      format!(
        "<a class={}{} href=\"{}\">{}</a>",
        if a.primary { "primary" } else { "secondary" },
        if i == 0 { " autofocus" } else { "" },
        escape(&a.href),
        escape(a.label)
      )
    })
    .collect();
  let failed = if matches!(badge, Badge::Failed) { " failed" } else { "" };

  format!(
    "<!doctype html><html lang=en><meta charset=utf-8><meta name=viewport content=\"width=device-width, initial-scale=1\"><meta name=color-scheme content=\"light dark\"><title>{title}</title><style>{STYLE}</style><main><span class=logo>{logo}</span><div class=card><span class=\"badge{failed}\">{badge}</span><h1>{heading}</h1><p>{message}</p><div class=actions>{links}</div></div></main>",
    title = escape(title),
    logo = svg(32, LOGO),
    badge = svg(22, badge.path()),
    heading = escape(heading),
    message = escape(message),
  )
}

/// A message written for logs (`the sign-in expired; start again`) as a sentence on a page.
pub(crate) fn sentence(message: &str) -> String {
  let mut chars = message.trim().chars();
  let Some(first) = chars.next() else { return String::new() };
  let mut text: String = first.to_uppercase().chain(chars).collect();
  if !text.ends_with(['.', '!', '?']) {
    text.push('.');
  }

  text
}

pub(crate) fn escape(text: &str) -> String {
  text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}
