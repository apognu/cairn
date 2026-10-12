use std::{
  io,
  sync::{
    OnceLock,
    mpsc::{self, Sender},
  },
  thread::{self, JoinHandle},
};

use ssr_rs::Ssr;

type Spawn = Box<dyn FnOnce() + Send>;

/// V8 protects its code tables with per-thread memory protection keys: every thread that touches V8
/// must descend from the thread that initialized the platform, or isolate creation segfaults.
/// One root thread initializes V8 and spawns every worker, wherever the request comes from.
static ROOT: OnceLock<Sender<Spawn>> = OnceLock::new();

pub(crate) fn spawn_v8_thread<F: FnOnce() + Send + 'static>(name: String, work: F) -> io::Result<JoinHandle<()>> {
  let root = ROOT.get_or_init(|| {
    let (tx, rx) = mpsc::channel::<Spawn>();

    thread::Builder::new()
      .name("v8-root".into())
      .spawn(move || {
        // ssr_rs panics if the platform is initialized twice; this runs exactly once per process.
        Ssr::create_platform();

        for spawn in rx {
          spawn();
        }
      })
      .expect("failed to start the V8 root thread");

    tx
  });

  let (reply_tx, reply_rx) = mpsc::channel();
  let spawn: Spawn = Box::new(move || {
    let _ = reply_tx.send(thread::Builder::new().name(name).spawn(work));
  });

  root.send(spawn).map_err(|_| io::Error::other("V8 root thread stopped"))?;
  reply_rx.recv().map_err(|_| io::Error::other("V8 root thread stopped"))?
}
