use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;
use tokio::sync::watch;

pub struct StatusChannels<T> {
    status: watch::Receiver<T>,
    channels: Arc<Mutex<BTreeMap<u32, Channel<T>>>>,
}

impl<T: Clone + Serialize + Send + Sync + 'static> StatusChannels<T> {
    pub fn new(mut updates: watch::Receiver<T>) -> Self {
        let status = updates.clone();
        let channels = Arc::new(Mutex::new(BTreeMap::<u32, Channel<T>>::new()));
        let subscribers = channels.clone();
        tauri::async_runtime::spawn(async move {
            while updates.changed().await.is_ok() {
                let mut subscribers = subscribers.lock().unwrap();
                // Read after locking channels to preserve initial-delivery ordering.
                let status = updates.borrow_and_update().clone();
                subscribers.retain(|_, channel| channel.send(status.clone()).is_ok());
            }
        });
        Self { status, channels }
    }

    pub fn subscribe(&self, channel: Channel<T>) -> tauri::Result<()> {
        let mut channels = self.channels.lock().unwrap();
        channel.send(self.status.borrow().clone())?;
        channels.insert(channel.id(), channel);
        Ok(())
    }

    pub fn unsubscribe(&self, channel_id: u32) {
        self.channels.lock().unwrap().remove(&channel_id);
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.channels.lock().unwrap().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    #[test]
    fn failed_update_delivery_removes_the_channel() {
        let (sender, updates) = watch::channel(0);
        let subscriptions = StatusChannels::new(updates);
        let first = AtomicBool::new(true);
        let (sent, received) = std::sync::mpsc::channel();
        let channel = Channel::new(move |_| {
            if first.swap(false, Ordering::SeqCst) {
                return Ok(());
            }
            sent.send(()).unwrap();
            Err(std::io::Error::other("closed channel").into())
        });
        assert_ok!(subscriptions.subscribe(channel));
        sender.send_replace(1);
        assert_ok!(received.recv_timeout(Duration::from_secs(2)));
        assert!(subscriptions.is_empty());
    }

    #[test]
    fn sender_closure_releases_forwarding_channels() {
        let (sender, updates) = watch::channel(0);
        let subscriptions = StatusChannels::new(updates);
        let (sent, received) = std::sync::mpsc::channel();
        assert_ok!(subscriptions.subscribe(Channel::new(move |_| {
            sent.send(()).unwrap();
            Ok(())
        })));
        assert_ok!(received.recv_timeout(Duration::from_secs(2)));
        drop(subscriptions);
        std::assert_matches!(
            received.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        );
        drop(sender);
        std::assert_matches!(
            received.recv_timeout(Duration::from_secs(2)),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected)
        );
    }

    #[derive(Serialize)]
    #[serde(transparent)]
    struct CheckedStatus {
        value: u8,
        #[serde(skip)]
        check: Option<Arc<dyn Fn() + Send + Sync>>,
    }

    impl Clone for CheckedStatus {
        fn clone(&self) -> Self {
            if let Some(check) = &self.check {
                check();
            }
            Self {
                value: self.value,
                check: self.check.clone(),
            }
        }
    }

    #[test]
    fn delivery_reads_status_while_holding_the_channel_lock() {
        let (sender, updates) = watch::channel(CheckedStatus {
            value: 0,
            check: None,
        });
        let subscriptions = StatusChannels::new(updates);
        let channels = Arc::downgrade(&subscriptions.channels);
        let checked = CheckedStatus {
            value: 1,
            check: Some(Arc::new(move || {
                assert_err!(channels.upgrade().unwrap().try_lock().map(|_| ()));
            })),
        };
        let (sent, received) = std::sync::mpsc::channel();
        let initial = sent.clone();
        assert_ok!(subscriptions.subscribe(Channel::new(move |body| {
            initial.send(body.deserialize::<u8>().unwrap()).unwrap();
            Ok(())
        })));
        assert_eq!(assert_ok!(received.recv_timeout(Duration::from_secs(2))), 0);
        sender.send_replace(checked);
        assert_eq!(assert_ok!(received.recv_timeout(Duration::from_secs(2))), 1);
        assert_ok!(subscriptions.subscribe(Channel::new(move |body| {
            sent.send(body.deserialize::<u8>().unwrap()).unwrap();
            Ok(())
        })));
        assert_eq!(assert_ok!(received.recv_timeout(Duration::from_secs(2))), 1);
    }
}
