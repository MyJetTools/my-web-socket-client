use futures::stream::SplitStream;
use hyper::{upgrade::Upgraded, Request};
use hyper_tungstenite::{tungstenite::Message, WebSocketStream};
use hyper_util::rt::TokioIo;

use rust_extensions::{date_time::DateTimeAsMicroseconds, Logger, StrOrString};

use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use super::{WsCallback, WsClientSettings, WsConnection};

pub struct WebSocketInner {
    pub reconnect_timeout: Duration,
    pub ping_interval: Duration,
    pub disconnect_timeout: Duration,
    pub send_timeout: Duration,
    pub working: AtomicBool,
}

impl WebSocketInner {
    pub fn is_working(&self) -> bool {
        self.working.load(std::sync::atomic::Ordering::Relaxed)
    }
}

pub struct WebSocketClient {
    pub inner: Arc<WebSocketInner>,
    name: String,
    pub logger: Arc<dyn Logger + Send + Sync + 'static>,
    pub settings: Arc<dyn WsClientSettings + Send + Sync + 'static>,
}

impl WebSocketClient {
    pub fn new(
        name: impl Into<StrOrString<'static>>,
        settings: Arc<dyn WsClientSettings + Send + Sync + 'static>,
        logger: Arc<dyn Logger + Send + Sync + 'static>,
    ) -> Self {
        Self {
            settings,
            logger,
            name: name.into().to_string(),
            inner: WebSocketInner {
                reconnect_timeout: Duration::from_secs(3),
                ping_interval: Duration::from_secs(3),
                disconnect_timeout: Duration::from_secs(9),
                send_timeout: Duration::from_secs(30),
                working: AtomicBool::new(true),
            }
            .into(),
        }
    }

    pub fn start<TWsCallback: WsCallback + Send + Sync + 'static>(
        &self,
        ping_message: Option<Message>,
        callback: Arc<TWsCallback>,
    ) {
        tokio::spawn(connection_loop(
            self.name.clone(),
            self.inner.clone(),
            self.settings.clone(),
            self.logger.clone(),
            callback,
            ping_message,
        ));
    }

    pub fn stop(&self) {
        self.inner
            .working
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }
}

async fn connection_loop<TWsCallback: WsCallback + Send + Sync + 'static>(
    name: String,
    inner: Arc<WebSocketInner>,
    endpoint: Arc<dyn WsClientSettings + Send + Sync + 'static>,
    logger: Arc<dyn Logger + Send + Sync + 'static>,
    ws_callback: Arc<TWsCallback>,
    ping_message: Option<Message>,
) {
    let mut connection_id = 0;
    while inner.is_working() {
        tokio::time::sleep(inner.reconnect_timeout).await;
        let url = endpoint.get_url().await;

        let mut log_ctx = HashMap::new();
        log_ctx.insert("url".to_string(), url.clone());
        log_ctx.insert("name".to_string(), name.clone());

        match super::connect::connect(url.as_str()).await {
            Ok(send_request) => {
                let (mut send_request, host_port) = send_request;
                let body = http_body_util::Full::new(hyper::body::Bytes::from(vec![]));
                let web_socket_key = generate_websocket_key();
                let req = Request::get(url)
                    .header("Host", host_port)
                    .header("Upgrade", "websocket")
                    .header("Connection", "Upgrade")
                    .header("Sec-WebSocket-Key", web_socket_key)
                    .header("Sec-WebSocket-Version", "13")
                    .body(body)
                    .unwrap();

                let result = send_request.send_request(req).await;

                let response = match result {
                    Ok(response) => response,
                    Err(err) => {
                        logger.write_warning(
                            "WebSocketConnectionLoop".to_string(),
                            format!("Executing initial get request. Err: {:?}", err),
                            Some(log_ctx),
                        );

                        continue;
                    }
                };

                if response.status() != 101 {
                    logger.write_warning(
                        "WebSocketConnectionLoop".to_string(),
                        format!("Initial get request. Status code: {:?}", response.status()),
                        Some(log_ctx),
                    );

                    continue;
                }

                let result = hyper::upgrade::on(response).await;

                let upgraded = match result {
                    Ok(result) => result,
                    Err(err) => {
                        logger.write_warning(
                            "WebSocketConnectionLoop".to_string(),
                            format!("Upgrading to WebSocket. Err: {:?}", err),
                            Some(log_ctx),
                        );

                        continue;
                    }
                };

                connection_id += 1;
                log_ctx.insert("connectionId".to_string(), connection_id.to_string());

                let web_socket = WebSocketStream::from_raw_socket(
                    TokioIo::new(upgraded),
                    hyper_tungstenite::tungstenite::protocol::Role::Client,
                    None,
                )
                .await;

                let (write, read) = futures::StreamExt::split(web_socket);

                let ws_connection =
                    Arc::new(WsConnection::new(connection_id, inner.send_timeout, write));

                let callback_spawned = ws_callback.clone();
                let ws_connection_spawned = ws_connection.clone();
                let on_connected_result = tokio::spawn(async move {
                    callback_spawned.on_connected(ws_connection_spawned).await;
                })
                .await;

                if on_connected_result.is_err() {
                    logger.write_error(
                        "WebSocketConnectionLoop".to_string(),
                        format!("Panic during on_connected"),
                        Some(log_ctx),
                    );
                    ws_connection.disconnect().await;
                    continue;
                }

                tokio::spawn(read_loop(
                    read,
                    ws_callback.clone(),
                    inner.clone(),
                    ws_connection.clone(),
                    inner.disconnect_timeout,
                    logger.clone(),
                    log_ctx.clone(),
                ));

                if let Some(ping_message) = ping_message.clone() {
                    ping_loop(&ws_connection, inner.clone(), ping_message.clone()).await;
                }

                let callback_spawned = ws_callback.clone();
                let ws_connection_spawned = ws_connection.clone();
                let _ = tokio::spawn(async move {
                    callback_spawned
                        .on_disconnected(ws_connection_spawned)
                        .await;
                })
                .await;
            }
            Err(err) => {
                logger.write_warning(
                    "WebSocketConnectionLoop".to_string(),
                    format!("Can not connect. Err: {:?}", err),
                    Some(log_ctx),
                );
            }
        }
    }
}

async fn read_loop<TWsCallback: WsCallback + Send + Sync + 'static>(
    mut read_stream: SplitStream<WebSocketStream<TokioIo<Upgraded>>>,
    ws_callback: Arc<TWsCallback>,
    inner: Arc<WebSocketInner>,
    ws_connection: Arc<WsConnection>,
    disconnect_timeout: Duration,
    logger: Arc<dyn Logger + Send + Sync + 'static>,
    log_ctx: HashMap<String, String>,
) {
    use futures::stream::StreamExt;
    while ws_connection.is_connected() {
        if !inner.is_working() {
            ws_connection.disconnect().await;
            break;
        }

        let result = tokio::time::timeout(disconnect_timeout, read_stream.next()).await;
        if result.is_err() {
            println!("read_loop. Timeout. Disconnecting... Err");
            ws_connection.disconnect().await;
            break;
        }

        let result = result.unwrap();

        if result.is_none() {
            ws_connection.disconnect().await;
            break;
        }

        let result = result.unwrap();

        match result {
            Ok(msg) => {
                let ws_callback_spawned = ws_callback.clone();
                let ws_connection_spawned = ws_connection.clone();
                ws_connection.update_last_read_time(DateTimeAsMicroseconds::now());
                let result = tokio::spawn(async move {
                    ws_callback_spawned
                        .on_data(ws_connection_spawned.clone(), msg)
                        .await
                })
                .await;

                if result.is_err() {
                    logger.write_fatal_error(
                        "WsSocketReadLoop".to_string(),
                        format!("Panic during handing data"),
                        Some(log_ctx),
                    );
                    ws_connection.disconnect().await;
                    break;
                }
            }
            Err(err) => {
                println!(
                    "Error reading loop. Can not get next message. Disconnecting... Err: {:?}",
                    err
                );
                ws_connection.disconnect().await;
            }
        }
    }

    println!("Exiting read loop");
}

async fn ping_loop(
    ws_connection: &Arc<WsConnection>,
    inner: Arc<WebSocketInner>,
    ping_message: Message,
) {
    while ws_connection.is_connected() {
        tokio::time::sleep(inner.ping_interval).await;

        let now = DateTimeAsMicroseconds::now();
        if now
            .duration_since(ws_connection.get_last_read_time())
            .as_positive_or_zero()
            > inner.disconnect_timeout
        {
            println!("Ping loop. Disconnecting. Timeout");
            break;
        }

        ws_connection.send_message(ping_message.clone()).await;
    }

    ws_connection.disconnect().await;
}

fn generate_websocket_key() -> String {
    use rand::Rng;
    use rust_extensions::base64::IntoBase64;
    let mut rng = rand::thread_rng();
    let mut key = [0u8; 16];
    rng.fill(&mut key);
    key.as_ref().into_base64()
}
