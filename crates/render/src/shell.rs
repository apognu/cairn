use maud::{DOCTYPE, PreEscaped, html};

/// Everything around the page body that the V8 render does not produce.
pub struct Shell<'a> {
  pub title: &'a str,
  pub site_name: &'a str,
  /// The HTML `ssr.js` produced.
  pub body: &'a str,
  /// The same input `ssr.js` was given, so the browser can hydrate exactly what was rendered.
  pub input: &'a str,
  pub stylesheets: &'a [String],
  pub script: &'a str,
  /// `#rrggbb`, already validated; it is written into a stylesheet.
  pub accent: &'a str,
}

// Applies the saved theme before the first paint, so a reader who chose dark never sees a light flash.
const THEME_INIT: &str = "try{var t=localStorage.getItem('yourdocs-theme');if(t==='light'||t==='dark')document.documentElement.dataset.theme=t}catch(e){}";

// Loads a page into the cache when the pointer rests on a link (and on keyboard focus), for browsers without speculation rules.
const PREFETCH: &str = "(function(){var seen={};function go(e){var a=e.target.closest&&e.target.closest('a[href]');if(!a||a.origin!==location.origin||a.pathname===location.pathname||seen[a.href])return;seen[a.href]=1;var l=document.createElement('link');l.rel='prefetch';l.href=a.href;document.head.append(l)}document.addEventListener('pointerover',go,{passive:true});document.addEventListener('focusin',go)})()";

// The sidebar keeps its scroll position across pages.
const NAV_SCROLL: &str = "(function(){var n=document.querySelector('.site-nav');if(!n)return;try{var y=sessionStorage.getItem('yourdocs-nav-scroll');if(y)n.scrollTop=+y;addEventListener('pagehide',function(){sessionStorage.setItem('yourdocs-nav-scroll',n.scrollTop)})}catch(e){}})()";

const SPECULATION_RULES: &str = r#"{"prefetch":[{"where":{"href_matches":"/*"},"eagerness":"moderate"}]}"#;

/// The complete HTML file of a page.
pub fn document(shell: &Shell) -> String {
  // `<` is escaped so the document can never close the script element.
  let input = shell.input.replace('<', "\\u003c");

  html! {
    (DOCTYPE)
    html lang="en" {
      head {
        meta charset="utf-8";
        meta name="viewport" content="width=device-width,initial-scale=1";
        meta name="color-scheme" content="light dark";
        title { (shell.title) " · " (shell.site_name) }
        meta property="og:title" content=(shell.title);
        meta property="og:site_name" content=(shell.site_name);
        meta property="og:type" content="article";
        script { (PreEscaped(THEME_INIT)) }
        @for href in shell.stylesheets {
          link rel="stylesheet" href=(href);
        }
        // After the stylesheets so it wins over the default. Dark mode derives a lighter shade of it in `site.css`.
        style { (PreEscaped(format!(":root{{--accent-base:{}}}", shell.accent))) }
        script type="speculationrules" { (PreEscaped(SPECULATION_RULES)) }
      }
      body {
        div id="root" { (PreEscaped(shell.body)) }
        script type="application/json" id="doc" { (PreEscaped(&input)) }
        script src=(shell.script) defer {}
        script { (PreEscaped(PREFETCH)) }
        script { (PreEscaped(NAV_SCROLL)) }
      }
    }
  }
  .into_string()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn escapes_the_title_and_cannot_be_broken_out_of_by_the_document() {
    let stylesheets = vec!["/assets/site.css".to_string()];
    let page = document(&Shell {
      title: "<b>Hi</b>",
      site_name: "Docs",
      body: "<p>body</p>",
      input: r#"{"text":"</script><script>alert(1)</script>"}"#,
      stylesheets: &stylesheets,
      script: "/assets/client.js",
      accent: "#0b6e4f",
    });

    assert!(page.contains("&lt;b&gt;Hi&lt;/b&gt; · Docs"), "{page}");
    assert!(page.contains("<p>body</p>"));
    assert!(!page.contains("</script><script>alert"));
    assert!(page.contains("rel=\"stylesheet\" href=\"/assets/site.css\""));
    assert!(page.contains("--accent-base:#0b6e4f"));
  }
}
