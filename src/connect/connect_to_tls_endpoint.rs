use std::sync::Arc;

use bytes::Bytes;
use http_body_util::Full;
use hyper::client::conn::http1::SendRequest;
use hyper_util::rt::TokioIo;
use my_tls::ROOT_CERT_STORE;
use tokio::net::TcpStream;

use crate::WsError;

pub async fn connect_to_tls_endpoint(
    url: &str,
) -> Result<(SendRequest<Full<Bytes>>, String), WsError> {
    use tokio_rustls::rustls::pki_types::ServerName;

    let host_port = super::extract_host_port(url).to_string();

    let tcp_stream = TcpStream::connect(host_port.as_str()).await?;

    let config = tokio_rustls::rustls::ClientConfig::builder()
        .with_root_certificates(ROOT_CERT_STORE.clone())
        .with_no_client_auth();

    let connector = tokio_rustls::TlsConnector::from(Arc::new(config));

    let host = super::extract_server_name(host_port.as_str());
    let domain = ServerName::try_from(host.to_string()).unwrap();

    let tls_stream = connector.connect(domain, tcp_stream).await?;

    let io = TokioIo::new(tls_stream);

    let handshake_result = hyper::client::conn::http1::handshake(io).await;

    match handshake_result {
        Ok((mut sender, conn)) => {
            let host_port_spawned = host_port.clone();
            tokio::task::spawn(async move {
                if let Err(err) = conn.with_upgrades().await {
                    println!(
                        "Https Connection to {} is failed: {:?}",
                        host_port_spawned, err
                    );
                }
            });

            sender.ready().await?;

            return Ok((sender, host_port));
        }
        Err(err) => {
            return Err(WsError::InvalidHttp1HandShake(format!("{}", err)));
        }
    }
}
