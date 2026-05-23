use std::time::Duration;

use eradic_common::ul::event::Event;
use tokio::sync::{mpsc, oneshot};
use tracing::{warn, info};

const ARTIM_TIMEOUT: Duration = Duration::from_secs(2);

/// ARTIM timer task. Fires [`Event::ArtimTimerExpired`] after [`ARTIM_TIMEOUT`] unless
/// `cancel` is sent or dropped first.
pub async fn artim_task(cancel: oneshot::Receiver<()>, event_tx: mpsc::Sender<Event>) {
    if tokio::time::timeout(ARTIM_TIMEOUT, cancel).await.is_err() {
        warn!("ARTIM timer expried");
        let _ = event_tx.send(Event::ArtimTimerExpired).await;
    }
}
