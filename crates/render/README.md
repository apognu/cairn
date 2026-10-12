# render

Renders ProseMirror documents to HTML with `ssr_rs` (embedded V8) on dedicated threads.
Not obvious: V8 isolates are `!Send`, so each pool thread owns its own instance.
