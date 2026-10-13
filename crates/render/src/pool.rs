use std::{
  sync::{
    Arc, Mutex,
    mpsc::{self, Receiver, SyncSender, TrySendError},
  },
  time::Duration,
};

use tokio::sync::oneshot;

use crate::{RenderError, engine, highlight::highlight_code_blocks, platform::spawn_v8_thread};

struct Job {
  doc: serde_json::Value,
  reply: oneshot::Sender<Result<String, RenderError>>,
}

/// Fixed set of threads, each owning one V8 isolate, fed through a bounded queue.
pub struct Pool {
  queue: SyncSender<Job>,
  timeout: Duration,
}

/// Tells a worker's watchdog that a render started (and how long it may take) or ended.
enum Watch {
  Arm(Duration),
  Disarm,
}

/// How much longer the requester waits than the worker lets a render run, so a stopped render is reported by the worker, as a timeout.
const GRACE: Duration = Duration::from_secs(2);

impl Pool {
  pub fn new(script: &str, threads: usize, queue_size: usize, timeout: Duration) -> Result<Self, RenderError> {
    let (queue, receiver) = mpsc::sync_channel::<Job>(queue_size);
    // Workers block on `recv` while holding this lock, which is fine: they are plain threads, not async tasks.
    let receiver = Arc::new(Mutex::new(receiver));
    let (ready_tx, ready_rx) = mpsc::channel();

    for n in 0..threads {
      let (script, receiver, ready) = (script.to_string(), receiver.clone(), ready_tx.clone());

      spawn_v8_thread(format!("render-{n}"), move || work(&script, &receiver, &ready, timeout)).map_err(|e| RenderError::Other(e.into()))?;
    }

    // Every worker reports once its isolate has loaded the script, so a broken bundle fails startup.
    for _ in 0..threads {
      ready_rx.recv().map_err(|e| RenderError::Other(e.into()))??;
    }

    Ok(Self { queue, timeout })
  }

  /// Asks the renderer whether a document would render: unknown blocks and component settings that do not fit their schema. Each
  /// problem is a sentence; an empty list means the document is fine. Much cheaper than rendering.
  pub async fn validate(&self, doc: &serde_json::Value) -> Result<Vec<String>, RenderError> {
    let answer = self.render(serde_json::json!({ "validate": true, "doc": doc })).await?;

    serde_json::from_str(&answer).map_err(|e| RenderError::Invalid(format!("the validation answer is not a list: {e}")))
  }

  /// Renders `{ "doc": …, "context": … }` (the shape `ssr.js` expects) to the page HTML.
  pub async fn render(&self, doc: serde_json::Value) -> Result<String, RenderError> {
    let (reply, response) = oneshot::channel();

    self.queue.try_send(Job { doc, reply }).map_err(|e| match e {
      TrySendError::Full(_) => RenderError::Busy,
      TrySendError::Disconnected(_) => RenderError::Invalid("render pool is shut down".into()),
    })?;

    // The worker stops a render that runs past the timeout and answers with `Timeout`; this wait is only a safety net behind it.
    match tokio::time::timeout(self.timeout + GRACE, response).await {
      Ok(Ok(result)) => result,
      Ok(Err(_)) => Err(RenderError::Invalid("render worker stopped".into())),
      Err(_) => Err(RenderError::Timeout),
    }
  }
}

fn work(script: &str, receiver: &Mutex<Receiver<Job>>, ready: &mpsc::Sender<Result<(), RenderError>>, timeout: Duration) {
  let (watch, watched) = mpsc::channel::<Watch>();
  let (done, acknowledged) = mpsc::channel::<()>();

  engine::run(
    script,
    |handle| match handle {
      Ok(handle) => {
        // The watchdog is a thread of its own, since the worker is busy inside V8 while a render runs.
        let started = spawn_v8_thread("render-watchdog".into(), move || watchdog(&handle, &watched, &done));
        let _ = ready.send(started.map(|_| ()).map_err(|e| RenderError::Other(e.into())));
      }
      Err(e) => {
        let _ = ready.send(Err(e));
      }
    },
    |render| loop {
      let job = match receiver.lock() {
        Ok(guard) => guard.recv(),
        Err(_) => return,
      };
      let Ok(job) = job else { return };

      let params = match serde_json::to_string(&job.doc) {
        Ok(params) => params,
        Err(e) => {
          let _ = job.reply.send(Err(RenderError::Other(e.into())));
          continue;
        }
      };

      let _ = watch.send(Watch::Arm(timeout));
      let outcome = render(&params).map(|html| highlight_code_blocks(&html));
      // Once the watchdog confirms it will not fire, no stop request can reach the next job.
      let _ = watch.send(Watch::Disarm);
      let _ = acknowledged.recv();

      // The requester may have given up and dropped its end.
      let _ = job.reply.send(outcome);
    },
  );
}

/// Stops the isolate when a render runs longer than it was given.
fn watchdog(handle: &v8::IsolateHandle, watched: &Receiver<Watch>, done: &mpsc::Sender<()>) {
  while let Ok(message) = watched.recv() {
    let Watch::Arm(limit) = message else { continue };

    match watched.recv_timeout(limit) {
      Ok(_) => {}
      Err(mpsc::RecvTimeoutError::Timeout) => {
        handle.terminate_execution();
        // The render notices, returns, and the worker disarms.
        if watched.recv().is_err() {
          return;
        }
      }
      Err(mpsc::RecvTimeoutError::Disconnected) => return,
    }

    if done.send(()).is_err() {
      return;
    }
  }
}

#[cfg(test)]
mod tests {
  use std::time::Instant;

  use super::*;

  fn script() -> String {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../packages/renderer/dist/ssr.js");
    std::fs::read_to_string(path).expect("ssr.js is missing, run `bun run build` first")
  }

  /// A bundle that renders by echoing, and spins forever for the input "spin".
  const SPINNING: &str = r#"var SSR = { render: function (input) { if (input.indexOf("spin") >= 0) { for (;;) {} } return "ok:" + input; } };"#;

  #[tokio::test]
  async fn a_runaway_render_is_stopped_within_the_timeout_and_the_worker_goes_on() {
    // One worker: if the spinning render kept it busy, the next job could never run.
    let pool = Pool::new(SPINNING, 1, 8, Duration::from_millis(300)).unwrap();

    let started = Instant::now();
    let stopped = pool.render(serde_json::json!("spin")).await;
    assert!(matches!(stopped, Err(RenderError::Timeout)), "{stopped:?}");
    assert!(started.elapsed() < Duration::from_secs(2), "stopped after {:?}", started.elapsed());

    // The same worker renders the next documents, more than once, and a second runaway is stopped as well.
    assert_eq!(pool.render(serde_json::json!("hello")).await.unwrap(), "ok:\"hello\"");
    assert!(matches!(pool.render(serde_json::json!("spin")).await, Err(RenderError::Timeout)));
    assert_eq!(pool.render(serde_json::json!("again")).await.unwrap(), "ok:\"again\"");
  }

  #[tokio::test]
  async fn every_fixture_renders() {
    let pool = Pool::new(&script(), 1, 8, Duration::from_secs(5)).unwrap();
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures");

    for entry in std::fs::read_dir(dir).unwrap() {
      let path = entry.unwrap().path();
      let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

      let html = pool
        .render(serde_json::json!({ "doc": doc, "context": { "page": { "title": "Fixture" } } }))
        .await
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));

      assert!(html.contains("class=\"doc-content\""), "{}", path.display());
    }
  }

  #[tokio::test]
  async fn new_tags_disappear_after_their_date() {
    let pool = Pool::new(&script(), 1, 8, Duration::from_secs(5)).unwrap();
    let doc: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/features.json")).unwrap();
    let render = |today: &'static str| {
      let doc = doc.clone();
      let pool = &pool;
      async move { pool.render(serde_json::json!({ "doc": doc, "context": { "today": today } })).await.unwrap() }
    };

    // The fixture has one tag dated 2099 and one dated 2000 in each component.
    assert_eq!(render("2026-10-12").await.matches("class=\"feature-new\"").count(), 2);
    assert_eq!(render("2099-12-31").await.matches("class=\"feature-new\"").count(), 2);
    assert_eq!(render("2100-01-01").await.matches("class=\"feature-new\"").count(), 0);
  }

  #[tokio::test]
  async fn renders_tab_labels() {
    let doc = serde_json::json!({ "doc": serde_json::from_str::<serde_json::Value>(include_str!("../../../fixtures/tabs.json")).unwrap() });
    let pool = Pool::new(&script(), 1, 8, Duration::from_secs(5)).unwrap();
    let html = pool.render(doc).await.unwrap();

    for expected in ["Install", "Linux", "macOS", "Use Homebrew."] {
      assert!(html.contains(expected), "missing {expected} in {html}");
    }
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn hundred_concurrent_renders() {
    let doc = serde_json::json!({ "doc": serde_json::from_str::<serde_json::Value>(include_str!("../../../fixtures/tabs.json")).unwrap() });
    let pool = Arc::new(Pool::new(&script(), 4, 256, Duration::from_secs(5)).unwrap());

    let started = Instant::now();
    let tasks: Vec<_> = (0..100)
      .map(|_| {
        let (pool, doc) = (pool.clone(), doc.clone());
        tokio::spawn(async move {
          let begin = Instant::now();
          let html = pool.render(doc).await.unwrap();
          (html, begin.elapsed())
        })
      })
      .collect();

    let mut slowest = Duration::ZERO;
    for task in tasks {
      let (html, took) = task.await.unwrap();
      assert!(html.contains("macOS"));
      slowest = slowest.max(took);
    }

    // Includes queueing behind the other 99 jobs, so this is stricter than the per-render goal.
    assert!(slowest < Duration::from_millis(50), "slowest {slowest:?}, total {:?}", started.elapsed());
  }
}
