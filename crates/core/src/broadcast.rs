use tokio::sync::broadcast::Receiver;

use crate::{PageId, ReleaseId, UserId};

/// Something that happened which other parts of the server (or other instances) may want to react to.
/// Events carry IDs, not content: they are hints, and the database stays the source of truth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
  /// A release became live.
  Published { release: ReleaseId },
  /// A build being handed to the publish target has sent `sent` of the `total` files the target lacked.
  PublishProgress { sent: u64, total: u64 },
  /// A page's lock was taken, given back or dropped: open editors read it again.
  LockChanged { page: PageId },
  /// Someone took the page from the editor of `from`, which must stop and keep its unsaved work.
  LockTakenOver { page: PageId, from: UserId },
}

/// Fans events out to subscribers. The first implementation lives in memory; a Postgres LISTEN/NOTIFY one comes with multi-instance setups.
pub trait Broadcast: Clone + Send + Sync + 'static {
  fn send(&self, event: Event);

  /// Events sent after this call. A subscriber that falls behind loses the oldest ones, which is fine for hints.
  fn subscribe(&self) -> Receiver<Event>;
}

/// `Broadcast` inside one process, on a tokio channel.
#[derive(Clone)]
pub struct LocalBroadcast {
  sender: tokio::sync::broadcast::Sender<Event>,
}

impl LocalBroadcast {
  pub fn new() -> Self {
    Self {
      sender: tokio::sync::broadcast::channel(64).0,
    }
  }
}

impl Default for LocalBroadcast {
  fn default() -> Self {
    Self::new()
  }
}

impl Broadcast for LocalBroadcast {
  fn send(&self, event: Event) {
    // No subscribers is not an error: nobody is listening right now.
    let _ = self.sender.send(event);
  }

  fn subscribe(&self) -> Receiver<Event> {
    self.sender.subscribe()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn subscribers_get_events_sent_after_they_subscribed() {
    let broadcast = LocalBroadcast::new();
    let release = ReleaseId::new();

    broadcast.send(Event::Published { release: ReleaseId::new() });
    let mut events = broadcast.subscribe();
    broadcast.send(Event::Published { release });

    assert_eq!(events.recv().await.unwrap(), Event::Published { release });
  }
}
