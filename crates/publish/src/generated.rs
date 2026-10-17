//! Files generated from the page list rather than rendered: sitemap, robots, llms.txt and the redirect rules.

use jiff::Timestamp;

/// A page of the site as the sitemap and `llms.txt` list it: a real page, or one the publisher generates (the Glossary).
pub(crate) struct Listed<'a> {
  pub title: &'a str,
  pub description: Option<&'a str>,
  /// The URL path (`/guide/`).
  pub url: &'a str,
  pub updated: Timestamp,
}

pub(crate) fn xml_escape(text: &str) -> String {
  text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

pub(crate) fn sitemap(site_url: &str, pages: &[Listed]) -> String {
  let entries: String = pages
    .iter()
    .map(|page| {
      format!(
        "  <url><loc>{}</loc><lastmod>{}</lastmod></url>\n",
        xml_escape(&format!("{site_url}{}", page.url)),
        page.updated.to_zoned(jiff::tz::TimeZone::UTC).date()
      )
    })
    .collect();

  format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n{entries}</urlset>\n")
}

pub(crate) fn robots(site_url: &str) -> String {
  format!("User-agent: *\nAllow: /\n\nSitemap: {site_url}/sitemap.xml\n")
}

/// A plain-text map of the site for language models: https://llmstxt.org. Each entry links the Markdown version of the page, with its
/// description when it has one.
pub(crate) fn llms_txt(site_name: &str, site_url: &str, pages: &[Listed]) -> String {
  let links: String = pages
    .iter()
    .map(|page| {
      let title = page.title.replace(['[', ']'], "");
      let note = page.description.map(|d| format!(": {d}")).unwrap_or_default();

      format!("- [{title}]({site_url}{}.md){note}\n", page.url.trim_end_matches('/'))
    })
    .collect();

  format!("# {site_name}\n\n## Pages\n\n{links}")
}

/// Redirect rules in the `_redirects` format of Cloudflare and Netlify: the site root, which has no page of its own,
/// then each `(from, to)` pair as a permanent redirect.
pub(crate) fn redirects(first_page_url: Option<&str>, rules: &[(String, String)]) -> String {
  let root = first_page_url.map(|url| format!("/ {url} 302\n")).unwrap_or_default();
  let rest: String = rules.iter().map(|(from, to)| format!("{from} {to} 301\n")).collect();

  root + &rest
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn generates_each_file() {
    let pages = [Listed {
      title: "Guide [v2]",
      description: None,
      url: "/guide/",
      updated: Timestamp::UNIX_EPOCH,
    }];

    let sitemap = sitemap("https://docs.example.com", &pages);
    assert!(sitemap.contains("<loc>https://docs.example.com/guide/</loc><lastmod>1970-01-01</lastmod>"), "{sitemap}");
    assert!(robots("https://docs.example.com").ends_with("Sitemap: https://docs.example.com/sitemap.xml\n"));
    assert_eq!(llms_txt("Docs", "https://d.io", &pages), "# Docs\n\n## Pages\n\n- [Guide v2](https://d.io/guide.md)\n");
    assert_eq!(redirects(Some("/guide/"), &[]), "/ /guide/ 302\n");
    assert_eq!(redirects(None, &[("/old/".into(), "/guide/".into())]), "/old/ /guide/ 301\n");
  }
}
