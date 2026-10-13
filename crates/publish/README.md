# publish

Releases and the `PublishTarget` enum (folder, self-hosted, Cloudflare Workers, S3). `Publisher` renders every page's latest revision in V8 and hands the files to the target, as the live site (`publish`) or as a preview that `promote` makes live later.
Not obvious: the folder targets write a whole release, then switch the `site` symlink in one rename, so a failed publish never touches the live site; S3 has no such switch (assets, pages, then deletions of what the previous publish wrote); targets are checked against their limits before anything is sent.
`republish_for_bundles` renders the live release again (same revisions, never drafts, the recorded API specs) when the JavaScript bundles changed.
`api::SourceFetcher` downloads public OpenAPI specs (HTTPS to public addresses only); every publish resolves API pages before creating a release. Cloudflare and S3 are tested against fakes, not real accounts.
