use http_body_util::Full;
use hyper::{body::Bytes, client::conn::http1::SendRequest};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

use crate::WsError;

pub async fn connect_to_http_endpoint(
    url: &str,
) -> Result<(SendRequest<Full<Bytes>>, String), WsError> {
    let host_port = super::extract_host_port(url).to_string();

    let tcp_stream = TcpStream::connect(host_port.as_str()).await?;

    let io = TokioIo::new(tcp_stream);
    let handshake_result = hyper::client::conn::http1::handshake(io).await;
    match handshake_result {
        Ok((mut sender, conn)) => {
            let remote_host = url.to_string();
            tokio::task::spawn(async move {
                if let Err(err) = conn.with_upgrades().await {
                    println!("Http Connection to {} is failed: {:?}", remote_host, err);
                }

                //Here
            });

            sender.ready().await?;
            return Ok((sender, host_port));
        }
        Err(err) => {
            return Err(WsError::InvalidHttp1HandShake(format!("{}", err)));
        }
    }
}
