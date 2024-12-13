use std::sync::Arc;

use futures::stream::{SplitSink, SplitStream};
use hyper_tungstenite::{tungstenite::Message, WebSocketStream};
use my_http_client::{http1::*, MyHttpClientConnector, MyHttpClientDisconnect};
use my_tls::tokio_rustls::client::TlsStream;

use crate::{MaybeTlsWebSocketStream, WebSocketInner, WsConnection};

pub enum MaybeTlsReadStream {
    NoTls(SplitStream<WebSocketStream<tokio::net::TcpStream>>),
    Tls(SplitStream<WebSocketStream<TlsStream<tokio::net::TcpStream>>>),
}

pub async fn connect_to_remote_endpoint<
    TStream: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + Sync + 'static,
    TConnector: MyHttpClientConnector<TStream> + Send + Sync + 'static,
>(
    inner: &Arc<WebSocketInner>,
    req: MyHttpRequest,
    connector: TConnector,
    create_stream: impl Fn(SplitSink<WebSocketStream<TStream>, Message>) -> MaybeTlsWebSocketStream,
) -> Result<
    (
        Arc<WsConnection>,
        SplitStream<WebSocketStream<TStream>>,
        Arc<dyn MyHttpClientDisconnect + Send + Sync + 'static>,
    ),
    String,
> {
    let my_http_client = my_http_client::http1::MyHttpClient::new(connector);

    let response = my_http_client.do_request(&req, inner.send_timeout).await;

    let response = match response {
        Ok(response) => response,
        Err(err) => {
            return Err(format!("{:?}", err));
        }
    };

    match response {
        MyHttpResponse::Response(response) => {
            return Err(format!(
                "Expecting websocket upgrade response. But got response: {:?}",
                response
            ));
        }
        MyHttpResponse::WebSocketUpgrade {
            stream,
            response: _,
            disconnection,
        } => {
            let web_socket = WebSocketStream::from_raw_socket(
                stream,
                hyper_tungstenite::tungstenite::protocol::Role::Client,
                None,
            )
            .await;

            let (write, read) = futures::StreamExt::split(web_socket);

            let web_socket_stream = create_stream(write);

            let ws_connection = Arc::new(WsConnection::new(web_socket_stream));

            Ok((ws_connection, read, disconnection))
        }
    }
}
