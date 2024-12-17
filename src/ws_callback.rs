use std::sync::Arc;

use hyper_tungstenite::tungstenite::Message;
use url_utils::UrlBuilder;

use super::WsConnection;

#[derive(Default)]
pub struct StartConnectionResult {
    pub headers: Option<Vec<(String, String)>>,
    pub url: Option<UrlBuilder>,
}

#[async_trait::async_trait]
pub trait WsCallback {
    async fn before_start_ws_connect(&self, url: String) -> Result<StartConnectionResult, String>;
    async fn on_connected(&self, ws_connection: Arc<WsConnection>);
    async fn on_disconnected(&self, ws_connection: Arc<WsConnection>);
    async fn on_data(&self, ws_connection: Arc<WsConnection>, data: Message);
}
