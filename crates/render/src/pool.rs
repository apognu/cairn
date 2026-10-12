use std::{
  sync::{
    Arc, Mutex,
    mpsc::{self, Receiver, SyncSender, TrySendError},
  },
  time::Duration,
};

use tokio::sync::oneshot;

use crate::{RenderError, Renderer, platform::spawn_v8_thread};

struct Job {
  doc: serde_json::Value,
  reply: oneshot::Sender<Result<String, RenderError>>,
}

/// Fixed set of threads, each owning one V8 isolate, fed through a bounded queue.
pub struct Pool {
  queue: SyncSender<Job>,
  timeout: Duration,
}

impl Pool {
  pub fn new(script: &str, threads: usize, queue_size: usize, timeout: Duration) -> Result<Self, RenderError> {
    let (queue, receiver) = mpsc::sync_channel::<Job>(queue_size);
    // Workers block on `recv` while holding this lock, which is fine: they are plain threads, not async tasks.
    let receiver = Arc::new(Mutex::new(receiver));
    let (ready_tx, ready_rx) = mpsc::channel();

    for n in 0..threads {
      let (script, receiver, ready) = (script.to_string(), receiver.clone(), ready_tx.clone());

      spawn_v8_thread(format!("render-{n}"), move || work(&script, &receiver, &ready)).map_err(|e| RenderError::Other(e.into()))?;
    }

    // Every worker reports once its isolate has loaded the script, so a broken bundle fails startup.
    for _ in 0..threads {
      ready_rx.recv().map_err(|e| RenderError::Other(e.into()))??;
    }

    Ok(Self { queue, timeout })
  }

  pub async fn render(&self, doc: serde_json::Value) -> Result<String, RenderError> {
    let (reply, response) = oneshot::channel();

    self.queue.try_send(Job { doc, reply }).map_err(|e| match e {
      TrySendError::Full(_) => RenderError::Busy,
      TrySendError::Disconnected(_) => RenderError::Invalid("render pool is shut down".into()),
    })?;

    // A timeout abandons the wait, but V8 keeps running the job until it returns.
    // TODO(M1.15): terminate the isolate on timeout, not needed for the spike
    match tokio::time::timeout(self.timeout, response).await {
      Ok(Ok(result)) => result,
      Ok(Err(_)) => Err(RenderError::Invalid("render worker stopped".into())),
      Err(_) => Err(RenderError::Timeout),
    }
  }
}

fn work(script: &str, receiver: &Mutex<Receiver<Job>>, ready: &mpsc::Sender<Result<(), RenderError>>) {
  let mut renderer = match Renderer::new(script) {
    Ok(renderer) => {
      let _ = ready.send(Ok(()));
      renderer
    }
    Err(e) => {
      let _ = ready.send(Err(e));
      return;
    }
  };

  loop {
    let job = match receiver.lock() {
      Ok(guard) => guard.recv(),
      Err(_) => return,
    };
    let Ok(job) = job else { return };

    // The requester may have timed out and dropped its end.
    let _ = job.reply.send(renderer.render(&job.doc));
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

  #[tokio::test]
  async fn renders_tab_labels() {
    let doc: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/tabs.json")).unwrap();
    let pool = Pool::new(&script(), 1, 8, Duration::from_secs(5)).unwrap();
    let html = pool.render(doc).await.unwrap();

    for expected in ["Install", "Linux", "macOS", "Use Homebrew."] {
      assert!(html.contains(expected), "missing {expected} in {html}");
    }
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn hundred_concurrent_renders() {
    let doc: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/tabs.json")).unwrap();
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
