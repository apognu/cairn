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
}

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
        @for href in shell.stylesheets {
          link rel="stylesheet" href=(href);
        }
      }
      body {
        div id="root" { (PreEscaped(shell.body)) }
        script type="application/json" id="doc" { (PreEscaped(&input)) }
        script src=(shell.script) defer {}
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
    });

    assert!(page.contains("&lt;b&gt;Hi&lt;/b&gt; · Docs"), "{page}");
    assert!(page.contains("<p>body</p>"));
    assert!(!page.contains("</script><script>alert"));
    assert!(page.contains("rel=\"stylesheet\" href=\"/assets/site.css\""));
  }
}
