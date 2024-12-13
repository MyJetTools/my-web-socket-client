use my_http_client::MyHttpClientError;

#[derive(Debug)]
pub enum WsError {
    IoError(std::io::Error),
    MyHttpClientConnector(MyHttpClientError),
    InvalidHttp1HandShake(String),
}

impl From<std::io::Error> for WsError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<MyHttpClientError> for WsError {
    fn from(err: MyHttpClientError) -> Self {
        Self::MyHttpClientConnector(err)
    }
}
