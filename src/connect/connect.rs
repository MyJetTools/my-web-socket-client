use bytes::Bytes;
use http_body_util::Full;
use hyper::client::conn::http1::SendRequest;

use crate::WsError;

pub async fn connect(url: &str) -> Result<(SendRequest<Full<Bytes>>, String), WsError> {
    if url.starts_with("wss") {
        return super::connect_to_tls_endpoint::connect_to_tls_endpoint(url).await;
    }
    return super::connect_to_http_endpoint::connect_to_http_endpoint(url).await;
}
