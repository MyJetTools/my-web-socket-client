use std::sync::atomic::AtomicBool;

use hyper_tungstenite::tungstenite::Message;

use rust_extensions::date_time::{AtomicDateTimeAsMicroseconds, DateTimeAsMicroseconds};
use tokio::sync::Mutex;

use super::MaybeTlsWebSocketStream;

pub struct WsConnection {
    inner: Mutex<Option<MaybeTlsWebSocketStream>>,
    is_connected: AtomicBool,
    last_read_time: AtomicDateTimeAsMicroseconds,
}

impl WsConnection {
    pub fn new(stream: MaybeTlsWebSocketStream) -> Self {
        Self {
            inner: Mutex::new(Some(stream)),
            is_connected: AtomicBool::new(true),
            last_read_time: AtomicDateTimeAsMicroseconds::now(),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn update_last_read_time(&self, now: DateTimeAsMicroseconds) {
        self.last_read_time.update(now);
    }

    pub fn get_last_read_time(&self) -> DateTimeAsMicroseconds {
        self.last_read_time.as_date_time()
    }

    pub async fn send_message(&self, message: Message) {
        let mut write_access = self.inner.lock().await;

        if let Some(single_threaded) = write_access.as_mut() {
            if !single_threaded.send(message).await {
                self.process_disconnect(&mut write_access).await;
            }
        }
    }

    pub async fn send_messages(&self, messages: impl Iterator<Item = Message>) {
        let mut write_access = self.inner.lock().await;

        if let Some(single_threaded) = write_access.as_mut() {
            for message in messages {
                if !single_threaded.send(message).await {
                    self.process_disconnect(&mut write_access).await;
                    break;
                }
            }
        }
    }

    pub async fn disconnect(&self) {
        let mut write_access = self.inner.lock().await;

        if write_access.is_some() {
            self.process_disconnect(&mut write_access).await;
        }
    }

    async fn process_disconnect(&self, single_threaded: &mut Option<MaybeTlsWebSocketStream>) {
        if let Some(inner) = single_threaded.as_mut() {
            inner.disconnect().await;
            self.is_connected
                .store(false, std::sync::atomic::Ordering::SeqCst);
        }

        *single_threaded = None;
    }
}
