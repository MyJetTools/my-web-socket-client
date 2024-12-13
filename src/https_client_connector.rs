use std::sync::Arc;

use my_http_client::MyHttpClientConnector;
use my_tls::tokio_rustls::client::TlsStream;
use rust_extensions::{remote_endpoint::RemoteEndpointOwned, StrOrString};
use tokio::{
    io::{ReadHalf, WriteHalf},
    net::TcpStream,
};

pub struct HttpsClientConnector {
    pub remote_endpoint: RemoteEndpointOwned,
    pub debug: bool,
    pub domain_name: Option<String>,
}

#[async_trait::async_trait]
impl MyHttpClientConnector<TlsStream<TcpStream>> for HttpsClientConnector {
    fn get_remote_host(&self) -> StrOrString {
        self.remote_endpoint.as_str().into()
    }

    fn is_debug(&self) -> bool {
        self.debug
    }

    async fn connect(&self) -> Result<TlsStream<TcpStream>, my_http_client::MyHttpClientError> {
        use my_tls::tokio_rustls::rustls::pki_types::ServerName;

        let host_port = self.remote_endpoint.get_host_port(Some(443));

        let tcp_stream = match TcpStream::connect(host_port.as_str()).await {
            Ok(tcp_stream) => tcp_stream,
            Err(err) => {
                return Err(
                    my_http_client::MyHttpClientError::CanNotConnectToRemoteHost(format!(
                        "{}",
                        err
                    )),
                )
            }
        };

        if self.debug {
            println!(
                "Connecting to TLS remote host: {}",
                self.remote_endpoint.as_str(),
            );
        }

        let config = my_tls::tokio_rustls::rustls::ClientConfig::builder()
            .with_root_certificates(my_tls::ROOT_CERT_STORE.clone())
            .with_no_client_auth();

        let connector = my_tls::tokio_rustls::TlsConnector::from(Arc::new(config));
        let domain = if let Some(domain_name) = self.domain_name.as_ref() {
            ServerName::try_from(domain_name.to_string()).unwrap()
        } else {
            ServerName::try_from(self.remote_endpoint.get_host().to_string()).unwrap()
        };

        if self.debug {
            println!("TLS Domain Name: {:?}", domain);
        }

        let tls_stream = connector
            .connect_with(domain, tcp_stream, |itm| {
                if self.debug {
                    println!("Debugging: {:?}", itm.alpn_protocol());
                }
            })
            .await;

        let tls_stream = match tls_stream {
            Ok(tls_stream) => tls_stream,
            Err(err) => {
                return Err(
                    my_http_client::MyHttpClientError::CanNotConnectToRemoteHost(format!(
                        "{}",
                        err
                    )),
                )
            }
        };

        return Ok(tls_stream);
    }

    fn reunite(
        read: ReadHalf<TlsStream<TcpStream>>,
        write: WriteHalf<TlsStream<TcpStream>>,
    ) -> TlsStream<TcpStream> {
        read.unsplit(write)
    }
}
