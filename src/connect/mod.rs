mod connect_to_http_endpoint;

mod connect;
mod connect_to_tls_endpoint;
pub use connect::*;

fn extract_host_port(url: &str) -> &str {
    let index = url.find("://");

    if index.is_none() {
        panic!("url {} does not have :// start", url);
    }

    let index = index.unwrap() + 3;

    let result = &url[index..];

    let index = result.find("/");

    match index {
        Some(index) => &result[..index],
        None => result,
    }
}

fn extract_server_name(host_port: &str) -> &str {
    let index = host_port.find(":");

    match index {
        Some(index) => &host_port[..index],
        None => host_port,
    }
}

#[cfg(test)]
mod test {
    use super::extract_host_port;

    #[test]
    fn test_no_path() {
        let src = "wss://www.google.com";

        let result = extract_host_port(src);

        assert_eq!(result, "www.google.com");
    }

    #[test]
    fn test_with_path() {
        let src = "wss://www.google.com/my";

        let result = extract_host_port(src);

        assert_eq!(result, "www.google.com");
    }

    #[test]
    fn test_with_port() {
        let src = "wss://www.google.com:443";

        let result = extract_host_port(src);

        assert_eq!(result, "www.google.com:443");
    }
}
