use my_http_client::{MyHttpClientConnector, MyHttpClientError};
use rust_extensions::remote_endpoint::*;
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
    fn get_remote_endpoint(&self) -> RemoteEndpoint {
        self.remote_endpoint.to_ref()
    }

    fn is_debug(&self) -> bool {
        self.debug
    }

    async fn connect(&self) -> Result<TcpStream, MyHttpClientError> {
        let host_port = self.remote_endpoint.get_host_port();

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
