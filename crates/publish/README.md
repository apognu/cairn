# publish

Releases and the `PublishTarget` enum. `Publisher` renders every page's latest revision in V8 and hands the files to the target.
Not obvious: the folder target writes a whole release, then switches the `site` symlink in one rename, so a failed publish never touches the live site.
