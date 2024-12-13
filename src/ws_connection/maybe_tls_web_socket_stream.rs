use std::time::Duration;

use hyper_tungstenite::WebSocketStream;

use super::WsConnectionInner;
use futures::stream::SplitSink;
use hyper_tungstenite::tungstenite::Message;
pub enum MaybeTlsWebSocketStream {
    NoTls(WsConnectionInner<tokio::net::TcpStream>),
    Tls(WsConnectionInner<my_tls::tokio_rustls::client::TlsStream<tokio::net::TcpStream>>),
}

impl MaybeTlsWebSocketStream {
    pub fn new_no_tls(
        id: i64,
        src: SplitSink<WebSocketStream<tokio::net::TcpStream>, Message>,
        send_timeout: Duration,
    ) -> Self {
        let inner = WsConnectionInner::new(send_timeout, src, id);
        Self::NoTls(inner)
    }

    pub fn new_tls(
        id: i64,
        src: SplitSink<
            WebSocketStream<my_tls::tokio_rustls::client::TlsStream<tokio::net::TcpStream>>,
            Message,
        >,
        send_timeout: Duration,
    ) -> Self {
        let inner = WsConnectionInner::new(send_timeout, src, id);
        Self::Tls(inner)
    }

    pub async fn send(&mut self, message: Message) -> bool {
        match self {
            Self::NoTls(inner) => inner.send(message).await,
            Self::Tls(inner) => inner.send(message).await,
        }
    }

    pub async fn disconnect(&mut self) {
        match self {
            Self::NoTls(inner) => inner.disconnect().await,
            Self::Tls(inner) => inner.disconnect().await,
        }
    }

    pub fn get_id(&self) -> i64 {
        match self {
            Self::NoTls(inner) => inner.id,
            Self::Tls(inner) => inner.id,
        }
    }
}

impl Into<MaybeTlsWebSocketStream>
    for (
        i64,
        SplitSink<WebSocketStream<tokio::net::TcpStream>, Message>,
        Duration,
    )
{
    fn into(self) -> MaybeTlsWebSocketStream {
        MaybeTlsWebSocketStream::new_no_tls(self.0, self.1, self.2)
    }
}

impl Into<MaybeTlsWebSocketStream>
    for (
        i64,
        SplitSink<
            WebSocketStream<my_tls::tokio_rustls::client::TlsStream<tokio::net::TcpStream>>,
            Message,
        >,
        Duration,
    )
{
    fn into(self) -> MaybeTlsWebSocketStream {
        MaybeTlsWebSocketStream::new_tls(self.0, self.1, self.2)
    }
}
