use std::time::Duration;

use futures::{stream::SplitSink, SinkExt};
use hyper_tungstenite::{tungstenite::Message, WebSocketStream};

pub struct WsConnectionInner<
    TStream: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + Sync + 'static,
> {
    pub stream: SplitSink<WebSocketStream<TStream>, Message>,
    pub send_timeout: Duration,
    pub id: i64,
}

impl<TStream: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + Sync + 'static>
    WsConnectionInner<TStream>
{
    pub fn new(
        send_timeout: Duration,
        stream: SplitSink<WebSocketStream<TStream>, Message>,
        id: i64,
    ) -> Self {
        Self {
            stream,
            send_timeout,
            id,
        }
    }

    pub async fn send(&mut self, message: Message) -> bool {
        let result = tokio::time::timeout(self.send_timeout, self.stream.send(message)).await;

        if result.is_err() {
            println!("Timeout while sending message. Connection: {}", self.id);
            return false;
        }

        let result = result.unwrap();

        if let Err(err) = result {
            println!(
                "Error while sending message. Connection: {}. Err:{}",
                self.id, err
            );
            return false;
        }

        true
    }

    pub async fn disconnect(&mut self) {
        if let Err(err) = self.stream.close().await {
            println!("Error while closing connection: {}", err);
        }
    }
}
