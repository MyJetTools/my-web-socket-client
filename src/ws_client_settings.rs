#[async_trait::async_trait]
pub trait WsClientSettings {
    async fn get_url(&self, client_name: &str) -> String;
}
