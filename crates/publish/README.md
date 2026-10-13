# publish

Releases and the `PublishTarget` enum. `Publisher` renders every page's latest revision in V8 and hands the files to the target.
Not obvious: the folder target writes a whole release, then switches the `site` symlink in one rename, so a failed publish never touches the live site.
`republish_for_bundles` renders the live release again (same revisions, never drafts, the recorded API specs) when the JavaScript bundles changed.
`api::SourceFetcher` downloads public OpenAPI specs (HTTPS to public addresses only); every publish resolves API pages before creating a release.
