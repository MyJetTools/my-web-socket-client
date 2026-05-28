# my-web-socket-client

WebSocket client for non-WASM Rust applications: automatic reconnection, heartbeat, and a callback-driven API.

## Cargo.toml

```toml
[dependencies]
my-web-socket-client = { tag = "max-tag", git = "https://github.com/MyJetTools/my-web-socket-client.git" }
```

## Usage

```rust
use std::sync::Arc;
use std::time::Duration;
use my_web_socket_client::*;
use my_web_socket_client::hyper_tungstenite::tungstenite::Message;

#[tokio::main]
async fn main() {
    my_web_socket_client::my_tls::install_default_crypto_providers();

    let client = WebSocketClient::new(
        Arc::new("MyClient".into()),
        Arc::new(MySettings),
        my_logger::LOGGER.clone(),
    )
    // streams where the upstream can legitimately stay silent for a while
    // (quiet instruments, servers that don't pong client pings) need a
    // higher disconnect timeout to avoid reconnect flapping:
    .with_disconnect_timeout(Duration::from_secs(30));

    client.start(Some(Message::Text("ping".into())), Arc::new(MyWsCallback));

    tokio::signal::ctrl_c().await.unwrap();
}
```

## Configurable timeouts

Builder-style methods (consume `self`, return `Self`) that must be called **before** `start()`:

| Method | Default | Description |
|---|---|---|
| `with_reconnect_timeout(Duration)` | 3s | Wait after a failed attempt (or a short-lived disconnect) before reconnecting |
| `with_ping_interval(Duration)` | 3s | How often the heartbeat message is sent |
| `with_disconnect_timeout(Duration)` | 9s | If no frame is received within this period (including pongs), the connection is dropped and re-established |
| `with_send_timeout(Duration)` | 30s | Max time to wait for a send to complete |
| `with_reconnect_delay_skip_threshold(Duration)` | 10s | How long a connection must have lived for its drop to count as a *healthy* disconnect, in which case the reconnect delay is skipped |

If a `with_*` method is not called the previous defaults apply, so this is backward-compatible.

## Reconnect behavior

- The **first** connection after `start()` is attempted immediately — there is no startup delay.
- Every **failed** connection attempt (bad URL, connect error, handshake/callback failure) always waits `reconnect_timeout` before retrying.
- When an established connection drops, the delay depends on how long it lived:
  - lived **≥ `reconnect_delay_skip_threshold`** → treated as a *healthy* disconnect (e.g. the server RST'ing a long-lived socket) and reconnected **immediately**, so no data window is lost;
  - lived **< threshold** → treated as a problem and the client waits `reconnect_timeout` before retrying.
