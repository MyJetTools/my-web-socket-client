use std::time::Duration;

use futures::{stream::SplitSink, SinkExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

pub struct WsConnectionSingleThreaded {
    pub stream: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    pub send_timeout: Duration,
    pub id: i64,
}

impl WsConnectionSingleThreaded {
    pub fn new(
        send_timeout: Duration,
        stream: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
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
