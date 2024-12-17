use std::sync::Arc;

use hyper_tungstenite::tungstenite::Message;

use super::WsConnection;

#[async_trait::async_trait]
pub trait WsCallback {
    async fn before_start_ws_connect(&self, url: String);
    async fn on_connected(&self, ws_connection: Arc<WsConnection>);
    async fn on_disconnected(&self, ws_connection: Arc<WsConnection>);
    async fn on_data(&self, ws_connection: Arc<WsConnection>, data: Message);
}
