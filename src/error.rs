#[derive(Debug)]
pub enum WsError {
    IoError(std::io::Error),
    HyperError(hyper::Error),
    InvalidHttp1HandShake(String),
}

impl From<std::io::Error> for WsError {
    fn from(err: std::io::Error) -> Self {
        WsError::IoError(err)
    }
}

impl From<hyper::Error> for WsError {
    fn from(err: hyper::Error) -> Self {
        WsError::HyperError(err)
    }
}
