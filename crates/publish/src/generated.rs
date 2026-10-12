//! Files generated from the page list rather than rendered: sitemap, robots, llms.txt and the redirect rules.

use yourdocs_core::Page;

fn xml_escape(text: &str) -> String {
  text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// `pages` pairs each page with its URL path (`/guide/`).
pub(crate) fn sitemap(site_url: &str, pages: &[(&Page, &str)]) -> String {
  let entries: String = pages
    .iter()
    .map(|(page, url)| {
      format!(
        "  <url><loc>{}</loc><lastmod>{}</lastmod></url>\n",
        xml_escape(&format!("{site_url}{url}")),
        page.updated_at.to_zoned(jiff::tz::TimeZone::UTC).date()
      )
    })
    .collect();

  format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n{entries}</urlset>\n")
}

pub(crate) fn robots(site_url: &str) -> String {
  format!("User-agent: *\nAllow: /\n\nSitemap: {site_url}/sitemap.xml\n")
}

/// A plain-text map of the site for language models: https://llmstxt.org
pub(crate) fn llms_txt(site_name: &str, site_url: &str, pages: &[(&Page, &str)]) -> String {
  let links: String = pages.iter().map(|(page, url)| format!("- [{}]({site_url}{url})\n", page.title.replace(['[', ']'], ""))).collect();

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
  use jiff::Timestamp;
  use yourdocs_core::{PageId, UserId};

  use super::*;

  fn page(title: &str) -> Page {
    Page {
      id: PageId::new(),
      kind: yourdocs_core::PageKind::Page,
      parent_id: None,
      position: "80".into(),
      slug: "a".into(),
      title: title.into(),
      external_url: None,
      path_override: None,
      in_header: false,
      header_position: None,
      description: None,
      created_by: UserId::new(),
      created_at: Timestamp::UNIX_EPOCH,
      updated_at: Timestamp::UNIX_EPOCH,
    }
  }

  #[test]
  fn generates_each_file() {
    let a = page("Guide [v2]");
    let pages = [(&a, "/guide/")];

    let sitemap = sitemap("https://docs.example.com", &pages);
    assert!(sitemap.contains("<loc>https://docs.example.com/guide/</loc><lastmod>1970-01-01</lastmod>"), "{sitemap}");
    assert!(robots("https://docs.example.com").ends_with("Sitemap: https://docs.example.com/sitemap.xml\n"));
    assert_eq!(llms_txt("Docs", "https://d.io", &pages), "# Docs\n\n## Pages\n\n- [Guide v2](https://d.io/guide/)\n");
    assert_eq!(redirects(Some("/guide/"), &[]), "/ /guide/ 302\n");
    assert_eq!(redirects(None, &[("/old/".into(), "/guide/".into())]), "/old/ /guide/ 301\n");
  }
}
