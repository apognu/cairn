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
  /// Fired when a worker takes the job: time spent waiting in the queue is not render time.
  started: oneshot::Sender<()>,
  reply: oneshot::Sender<Result<String, RenderError>>,
}

/// Fixed set of threads, each owning one V8 isolate, fed through a bounded queue.
pub struct Pool {
  script: Arc<str>,
  threads: usize,
  queue_size: usize,
  timeout: Duration,
  /// Started by the first job of a lazy pool, so a server that only edits never pays for V8.
  workers: tokio::sync::OnceCell<Workers>,
}

struct Workers {
  queue: SyncSender<Job>,
  /// How long a job may wait for a worker: every job ahead of it running to its limit. Only a pool whose workers are all stuck reaches it.
  queue_wait: Duration,
}

/// Tells a worker's watchdog that a render started (and how long it may take) or ended.
enum Watch {
  Arm(Duration),
  Disarm,
}

/// How much longer the requester waits than the worker lets a render run, so a stopped render is reported by the worker, as a timeout.
const GRACE: Duration = Duration::from_secs(2);

impl Pool {
  /// Starts the workers now: a broken bundle fails here.
  pub fn new(script: &str, threads: usize, queue_size: usize, timeout: Duration) -> Result<Self, RenderError> {
    let workers = start(script, threads, queue_size, timeout)?;

    Ok(Self {
      workers: tokio::sync::OnceCell::new_with(Some(workers)),
      ..Self::lazy(script, threads, queue_size, timeout)
    })
  }

  /// Starts the workers on the first job instead; a broken bundle fails that job (and the next ones, which try again).
  pub fn lazy(script: &str, threads: usize, queue_size: usize, timeout: Duration) -> Self {
    Self {
      script: script.into(),
      threads,
      queue_size,
      timeout,
      workers: tokio::sync::OnceCell::new(),
    }
  }

  async fn workers(&self) -> Result<&Workers, RenderError> {
    self
      .workers
      .get_or_try_init(|| async {
        let (script, threads, queue_size, timeout) = (self.script.clone(), self.threads, self.queue_size, self.timeout);
        // Loading the bundle in every isolate takes a moment of blocking work.
        tokio::task::spawn_blocking(move || start(&script, threads, queue_size, timeout))
          .await
          .map_err(|e| RenderError::Other(e.into()))?
      })
      .await
  }

  /// Asks the renderer whether a document would render: unknown blocks and component settings that do not fit their schema. Each
  /// problem is a sentence; an empty list means the document is fine. Much cheaper than rendering.
  pub async fn validate(&self, doc: &serde_json::Value) -> Result<Vec<String>, RenderError> {
    let answer = self.render(serde_json::json!({ "validate": true, "doc": doc })).await?;

    serde_json::from_str(&answer).map_err(|e| RenderError::Other(anyhow::anyhow!("the validation answer is not a list: {e}")))
  }

  /// Renders `{ "doc": …, "context": … }` (the shape `ssr.js` expects) to the page HTML.
  pub async fn render(&self, doc: serde_json::Value) -> Result<String, RenderError> {
    let (reply, response) = oneshot::channel();
    let (started, picked) = oneshot::channel();

    let workers = self.workers().await?;
    workers.queue.try_send(Job { doc, started, reply }).map_err(|e| match e {
      TrySendError::Full(_) => RenderError::Busy,
      TrySendError::Disconnected(_) => RenderError::Other(anyhow::anyhow!("render pool is shut down")),
    })?;

    match tokio::time::timeout(workers.queue_wait, picked).await {
      Ok(Ok(())) => {}
      Ok(Err(_)) => return Err(RenderError::Other(anyhow::anyhow!("render worker stopped"))),
      Err(_) => return Err(RenderError::Timeout),
    }

    // The worker stops a render that runs past the timeout and answers with `Timeout`; this wait is only a safety net behind it.
    match tokio::time::timeout(self.timeout + GRACE, response).await {
      Ok(Ok(result)) => result,
      Ok(Err(_)) => Err(RenderError::Other(anyhow::anyhow!("render worker stopped"))),
      Err(_) => Err(RenderError::Timeout),
    }
  }
}

fn start(script: &str, threads: usize, queue_size: usize, timeout: Duration) -> Result<Workers, RenderError> {
  let (queue, receiver) = mpsc::sync_channel::<Job>(queue_size);
  // Workers block on `recv` while holding this lock, which is fine: they are plain threads, not async tasks.
  let receiver = Arc::new(Mutex::new(receiver));
  let (ready_tx, ready_rx) = mpsc::channel();

  for n in 0..threads {
    let (script, receiver, ready) = (script.to_string(), receiver.clone(), ready_tx.clone());

    spawn_v8_thread(format!("render-{n}"), move || work(&script, &receiver, &ready, timeout)).map_err(|e| RenderError::Other(e.into()))?;
  }

  // Every worker reports once its isolate has loaded the script, so a broken bundle fails the start.
  for _ in 0..threads {
    ready_rx.recv().map_err(|e| RenderError::Other(e.into()))??;
  }

  let queue_wait = (timeout + GRACE) * u32::try_from(queue_size.div_ceil(threads.max(1)) + 1).unwrap_or(u32::MAX);

  Ok(Workers { queue, queue_wait })
}

fn work(script: &str, receiver: &Mutex<Receiver<Job>>, ready: &mpsc::Sender<Result<(), RenderError>>, timeout: Duration) {
  // A render that ran out of memory leaves its isolate full: the worker starts over with a fresh one.
  while serve_jobs(script, receiver, ready, timeout) == Next::FreshIsolate {}
}

#[derive(PartialEq)]
enum Next {
  FreshIsolate,
  Stop,
}

fn serve_jobs(script: &str, receiver: &Mutex<Receiver<Job>>, ready: &mpsc::Sender<Result<(), RenderError>>, timeout: Duration) -> Next {
  let (watch, watched) = mpsc::channel::<Watch>();
  let (done, acknowledged) = mpsc::channel::<()>();
  let mut next = Next::Stop;

  engine::run(
    script,
    |handle| match handle {
      Ok(handle) => {
        // The watchdog is a thread of its own, since the worker is busy inside V8 while a render runs. It ends with this isolate, when
        // `watch` is dropped.
        let started = spawn_v8_thread("render-watchdog".into(), move || watchdog(&handle, &watched, &done));
        // Only the first isolate's report is awaited; later ones go nowhere.
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

      let _ = job.started.send(());
      let _ = watch.send(Watch::Arm(timeout));
      let outcome = render(&params).map(|html| highlight_code_blocks(&html));
      // Once the watchdog confirms it will not fire, no stop request can reach the next job.
      let _ = watch.send(Watch::Disarm);
      let _ = acknowledged.recv();

      let out_of_memory = matches!(outcome, Err(RenderError::OutOfMemory));
      // The requester may have given up and dropped its end.
      let _ = job.reply.send(outcome);

      if out_of_memory {
        next = Next::FreshIsolate;
        return;
      }
    },
  );

  next
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
        // The render may have finished just before the stop was asked for. A stop requested while no script runs stays pending and
        // would end the next job at once, so it is withdrawn now that the render is over (the worker waits for `done` before going on).
        handle.cancel_terminate_execution();
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
  #[tokio::test]
  async fn a_lazy_pool_starts_with_its_first_job_and_reports_a_broken_bundle_there() {
    let broken = Pool::lazy("this is not javascript", 1, 8, Duration::from_secs(5));
    assert!(broken.render(serde_json::json!("x")).await.is_err());

    let pool = Pool::lazy(SPINNING, 1, 8, Duration::from_secs(5));
    assert_eq!(pool.render(serde_json::json!("x")).await.unwrap(), "ok:\"x\"");
  }

  const SPINNING: &str = r#"var SSR = { render: function (input) { if (input.indexOf("spin") >= 0) { for (;;) {} } return "ok:" + input; } };"#;

  #[tokio::test]
  async fn time_spent_waiting_in_the_queue_is_not_render_time() {
    // Each render takes 200ms, within its 300ms; fifteen behind one worker wait longer than the timeout and its grace together.
    const SLOW: &str = r#"var SSR = { render: function (input) { var end = Date.now() + 200; while (Date.now() < end) {} return input; } };"#;
    let pool = Arc::new(Pool::new(SLOW, 1, 32, Duration::from_millis(300)).unwrap());

    let mut renders = tokio::task::JoinSet::new();
    for n in 0..15 {
      let pool = pool.clone();
      renders.spawn(async move { pool.render(serde_json::json!(n)).await });
    }
    while let Some(result) = renders.join_next().await {
      assert!(result.unwrap().is_ok());
    }
  }

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
  async fn a_render_that_allocates_without_end_fails_alone() {
    // Keeps every array it makes, so nothing can be collected; the timeout is long enough that memory runs out first.
    const GREEDY: &str =
      r#"var SSR = { render: function (input) { if (input.indexOf("greedy") >= 0) { var kept = []; for (;;) { kept.push(new Array(100000).fill(kept.length)); } } return "ok:" + input; } };"#;
    let pool = Pool::new(GREEDY, 1, 8, Duration::from_secs(60)).unwrap();

    let stopped = pool.render(serde_json::json!("greedy")).await;
    assert!(matches!(stopped, Err(RenderError::OutOfMemory)), "{stopped:?}");

    // The one worker goes on with a fresh isolate, which stops a second greedy render the same way.
    assert_eq!(pool.render(serde_json::json!("hello")).await.unwrap(), "ok:\"hello\"");
    assert!(matches!(pool.render(serde_json::json!("greedy")).await, Err(RenderError::OutOfMemory)));
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

    // The slowest render waits behind the other 99, so the bound is the per-render budget times the queue depth per thread;
    // a fixed bound on it only held on machines with more cores than the 4-vCPU CI runners.
    let budget = Duration::from_millis(10) * 100 / 4;
    assert!(slowest < budget, "slowest {slowest:?}, total {:?}, budget {budget:?}", started.elapsed());
  }
}
