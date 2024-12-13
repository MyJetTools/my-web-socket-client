use rust_extensions::StrOrString;

#[async_trait::async_trait]
pub trait WsClientSettings {
    async fn get_url(&self) -> String;
    async fn apply_headers_in_init(
        &self,
    ) -> Option<Vec<(StrOrString<'static>, StrOrString<'static>)>>;
}
