use std::sync::Arc;

use tokio_tungstenite::tungstenite::Message;

use super::WsConnection;

#[async_trait::async_trait]
pub trait WsCallback {
    async fn on_connected(&self, ws_connection: Arc<WsConnection>);
    async fn on_disconnected(&self, ws_connection: Arc<WsConnection>);
    async fn on_data(&self, ws_connection: Arc<WsConnection>, data: Message);
}
