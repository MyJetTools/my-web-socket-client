use my_http_client::{MyHttpClientConnector, MyHttpClientError};
use rust_extensions::{remote_endpoint::RemoteEndpointOwned, StrOrString};
use tokio::{
    io::{ReadHalf, WriteHalf},
    net::TcpStream,
};

pub struct HttpClientConnector {
    pub remote_endpoint: RemoteEndpointOwned,
    pub debug: bool,
}

#[async_trait::async_trait]
impl MyHttpClientConnector<TcpStream> for HttpClientConnector {
    fn get_remote_host(&self) -> StrOrString {
        self.remote_endpoint.as_str().into()
    }

    fn is_debug(&self) -> bool {
        self.debug
    }

    async fn connect(&self) -> Result<TcpStream, MyHttpClientError> {
        let host = self.remote_endpoint.get_host();

        let port = self.remote_endpoint.get_port().unwrap_or(80);

        let host_port = format!("{}:{}", host, port);

        match TcpStream::connect(host_port.as_str()).await {
            Ok(tcp_stream) => Ok(tcp_stream),
            Err(err) => Err(
                my_http_client::MyHttpClientError::CanNotConnectToRemoteHost(format!(
                    "{}. Err:{}",
                    self.remote_endpoint.as_str(),
                    err
                )),
            ),
        }
    }

    fn reunite(read: ReadHalf<TcpStream>, write: WriteHalf<TcpStream>) -> TcpStream {
        read.unsplit(write)
    }
}
