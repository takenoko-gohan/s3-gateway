use futures_util::future::join;
use std::net::SocketAddr;

mod server;

type Error = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt().init();

    let gateway = server::Server::builder()
        .addr(SocketAddr::from(([0, 0, 0, 0], 80)))
        .build();
    let management = server::Server::builder()
        .addr(SocketAddr::from(([0, 0, 0, 0], 8080)))
        .build();

    let (gateway_result, management_result) = join(gateway, management).await;

    gateway_result.unwrap_or_else(|e| tracing::error!("Gateway server failed: {}", e));
    management_result.unwrap_or_else(|e| tracing::error!("Management server failed: {}", e));

    Ok(())
}
