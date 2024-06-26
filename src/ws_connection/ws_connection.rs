use std::{sync::atomic::AtomicBool, time::Duration};

use futures::stream::SplitSink;
use hyper::upgrade::Upgraded;
use hyper_tungstenite::{tungstenite::Message, WebSocketStream};
use hyper_util::rt::TokioIo;
use rust_extensions::date_time::{AtomicDateTimeAsMicroseconds, DateTimeAsMicroseconds};
use tokio::sync::Mutex;

use super::WsConnectionSingleThreaded;

pub struct WsConnection {
    single_threaded: Mutex<Option<WsConnectionSingleThreaded>>,
    is_connected: AtomicBool,
    last_read_time: AtomicDateTimeAsMicroseconds,
}

impl WsConnection {
    pub fn new(
        id: i64,
        send_timeout: Duration,
        stream: SplitSink<WebSocketStream<TokioIo<Upgraded>>, Message>,
    ) -> Self {
        Self {
            single_threaded: Mutex::new(Some(WsConnectionSingleThreaded::new(
                send_timeout,
                stream,
                id,
            ))),
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
        let mut write_access = self.single_threaded.lock().await;

        if let Some(single_threaded) = write_access.as_mut() {
            if !single_threaded.send(message).await {
                self.process_disconnect(&mut write_access).await;
            }
        }
    }

    pub async fn disconnect(&self) {
        let mut write_access = self.single_threaded.lock().await;

        if write_access.is_some() {
            self.process_disconnect(&mut write_access).await;
        }
    }

    async fn process_disconnect(&self, single_threaded: &mut Option<WsConnectionSingleThreaded>) {
        if let Some(single_threaded) = single_threaded.as_mut() {
            single_threaded.disconnect().await;
            self.is_connected
                .store(false, std::sync::atomic::Ordering::SeqCst);
        }

        *single_threaded = None;
    }
}
