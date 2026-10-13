# publish

Releases and the `PublishTarget` enum. `Publisher` renders every page's latest revision in V8 and hands the files to the target.
Not obvious: the folder target writes a whole release, then switches the `site` symlink in one rename, so a failed publish never touches the live site.
`republish_for_bundles` renders the live release again (same revisions, never drafts) when the JavaScript bundles changed since the last live release.
`api::SourceFetcher` downloads public OpenAPI specs (URLs, GitHub files and release assets) without the GitHub API; it refuses private addresses and non-HTTPS.
