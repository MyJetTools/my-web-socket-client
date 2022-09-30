use futures::stream::SplitStream;
use futures::StreamExt;
use rust_extensions::{date_time::DateTimeAsMicroseconds, Logger};

use std::{collections::HashMap, sync::Arc, time::Duration};

use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

use super::{WsCallback, WsClientSettings, WsConnection};

#[derive(Clone, Copy)]
pub struct WebSocketTimeouts {
    pub reconnect_timeout: Duration,
    pub ping_interval: Duration,
    pub disconnect_timeout: Duration,
    pub send_timeout: Duration,
}

pub struct WebSocketClient {
    pub timeouts: WebSocketTimeouts,
    name: String,
    pub logger: Arc<dyn Logger + Send + Sync + 'static>,
    pub settings: Arc<dyn WsClientSettings + Send + Sync + 'static>,
}

impl WebSocketClient {
    pub fn new(
        name: String,
        settings: Arc<dyn WsClientSettings + Send + Sync + 'static>,
        logger: Arc<dyn Logger + Send + Sync + 'static>,
    ) -> Self {
        Self {
            settings,
            logger,
            name,
            timeouts: WebSocketTimeouts {
                reconnect_timeout: Duration::from_secs(3),
                ping_interval: Duration::from_secs(3),
                disconnect_timeout: Duration::from_secs(9),
                send_timeout: Duration::from_secs(30),
            },
        }
    }

    pub fn start<TWsCallback: WsCallback + Send + Sync + 'static>(
        &self,
        ping_message: Message,
        callback: Arc<TWsCallback>,
    ) {
        tokio::spawn(connection_loop(
            self.name.clone(),
            self.timeouts,
            self.settings.clone(),
            self.logger.clone(),
            callback,
            ping_message,
        ));
    }
}

async fn connection_loop<TWsCallback: WsCallback + Send + Sync + 'static>(
    name: String,
    timeouts: WebSocketTimeouts,
    endpoint: Arc<dyn WsClientSettings + Send + Sync + 'static>,
    logger: Arc<dyn Logger + Send + Sync + 'static>,
    ws_callback: Arc<TWsCallback>,
    ping_message: Message,
) {
    let mut connection_id = 0;
    loop {
        tokio::time::sleep(timeouts.reconnect_timeout).await;
        let url = endpoint.get_url().await;

        let mut log_ctx = HashMap::new();
        log_ctx.insert("url".to_string(), url.clone());
        log_ctx.insert("name".to_string(), name.clone());

        match tokio_tungstenite::connect_async(url).await {
            Ok((stream, _)) => {
                connection_id += 1;
                log_ctx.insert("connectionId".to_string(), connection_id.to_string());

                let (write, read) = futures::StreamExt::split(stream);

                let ws_connection = Arc::new(WsConnection::new(
                    connection_id,
                    timeouts.send_timeout,
                    write,
                ));

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
                    ws_connection.clone(),
                    timeouts.disconnect_timeout,
                    logger.clone(),
                    log_ctx.clone(),
                ));

                ping_loop(
                    &ws_connection,
                    timeouts.ping_interval,
                    timeouts.disconnect_timeout,
                    ping_message.clone(),
                )
                .await;

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
    mut read_stream: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    ws_callback: Arc<TWsCallback>,
    ws_connection: Arc<WsConnection>,
    disconnect_timeout: Duration,
    logger: Arc<dyn Logger + Send + Sync + 'static>,
    log_ctx: HashMap<String, String>,
) {
    while ws_connection.is_connected() {
        let result = tokio::time::timeout(disconnect_timeout, read_stream.next()).await;
        if result.is_err() {
            println!("read_loop. Timeout. Discoonecting... Err");
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
                    "Error reading loop. Can not get next message. Discoonecting... Err: {:?}",
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
    ping_interval: Duration,
    disconnect_timeout: Duration,
    ping_message: Message,
) {
    while ws_connection.is_connected() {
        tokio::time::sleep(ping_interval).await;

        let now = DateTimeAsMicroseconds::now();
        if now
            .duration_since(ws_connection.get_last_read_time())
            .as_positive_or_zero()
            > disconnect_timeout
        {
            ws_connection.disconnect().await;
            println!("Ping loop. Disconnecting. Timeout");
            break;
        }

        ws_connection.send_message(ping_message.clone()).await;
    }
}
