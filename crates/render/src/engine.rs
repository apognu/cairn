//! One V8 isolate with the bundle loaded, living for the length of a call to `run`.
//!
//! Everything V8 hands out borrows from the isolate, so the isolate cannot be returned in a struct; instead `run` owns it for the whole life of
//! a worker and hands the caller a function that renders one input.

use crate::RenderError;

/// V8's own limit for one isolate. It is a hard ceiling: V8 stops the whole process when a render reaches it, so it sits far above what a
/// page needs (a few tens of megabytes) and only stops a document that allocates without end.
pub(crate) const HEAP_LIMIT: usize = 384 * 1024 * 1024;
const HEAP_START: usize = 16 * 1024 * 1024;

/// Loads `script` (which must define `SSR` with a `render` function), tells the caller how to stop the isolate from another thread, and
/// calls `serve` with a function that renders one string input to a string output. The isolate is dropped when `serve` returns.
pub(crate) fn run(script: &str, ready: impl FnOnce(Result<v8::IsolateHandle, RenderError>), serve: impl FnOnce(&mut dyn FnMut(&str) -> Result<String, RenderError>)) {
  let mut isolate = v8::Isolate::new(v8::CreateParams::default().heap_limits(HEAP_START, HEAP_LIMIT));
  let handle = isolate.thread_safe_handle();

  let scope = &mut v8::HandleScope::new(&mut isolate);
  let context = v8::Context::new(scope, Default::default());
  let scope = &mut v8::ContextScope::new(scope, context);

  let Some(render) = load(scope, script) else {
    ready(Err(RenderError::Other(anyhow::anyhow!("the render bundle did not load or has no SSR.render function"))));
    return;
  };
  ready(Ok(handle.clone()));

  let mut render_one = |input: &str| -> Result<String, RenderError> {
    // Handles made for one render are released when it ends, not kept for the life of the isolate.
    let scope = &mut v8::HandleScope::new(scope);
    let argument = v8::String::new(scope, input).ok_or_else(|| RenderError::Invalid("the input is too large".into()))?;
    let undefined = v8::undefined(scope).into();

    let scope = &mut v8::TryCatch::new(scope);
    match render.call(scope, undefined, &[argument.into()]) {
      Some(result) => Ok(result.to_rust_string_lossy(scope)),
      // The watchdog stopped it: a render that ran past its time.
      None if scope.has_terminated() => {
        handle.cancel_terminate_execution();
        Err(RenderError::Timeout)
      }
      None => {
        let message = scope.exception().map(|e| e.to_rust_string_lossy(scope)).unwrap_or_else(|| "the render threw".into());
        Err(RenderError::Invalid(message))
      }
    }
  };

  serve(&mut render_one);
}

fn load<'s>(scope: &mut v8::ContextScope<'_, v8::HandleScope<'s>>, script: &str) -> Option<v8::Local<'s, v8::Function>> {
  // The bundle is an IIFE assigned to `SSR`; evaluating `SSR` after it yields the exported object.
  let source = v8::String::new(scope, &format!("{script};SSR"))?;
  let exports = v8::Script::compile(scope, source, None)?.run(scope)?.to_object(scope)?;
  let name = v8::String::new(scope, "render")?;

  exports.get(scope, name.into())?.try_into().ok()
}
