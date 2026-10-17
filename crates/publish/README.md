# publish

Releases and the `PublishTarget` enum (folder, self-hosted, Cloudflare Workers, S3). `Publisher` renders every page's latest revision in V8 and hands the files to the target, as the live site (`publish`) or as a preview that `promote` makes live later.
Not obvious: the folder targets write a whole release, then switch the `site` symlink in one rename, so a failed publish never touches the live site; S3 has no such switch (assets, pages, then deletions of what the previous publish wrote); targets are checked against their limits before anything is sent.
`rebuild_from` renders a release again from its frozen state (same revisions, never drafts, the recorded API specs): `republish_for_bundles` uses it when the JavaScript bundles changed, and `rollback` when the target no longer holds the release. `promote` refuses a preview whose base or staging moved on; full `publish` calls waiting for a running build become one build.
`api::SourceFetcher` downloads public OpenAPI specs (HTTPS to public addresses only); every publish resolves API pages before creating a release. Cloudflare and S3 are tested against fakes, not real accounts.
Each page also gets a 1200×630 link preview PNG (`card.rs`: SVG rasterized by resvg with the bundled Hanken Grotesk, OFL, no system fonts), cached in `renders/` beside the HTML and named by its hash.
