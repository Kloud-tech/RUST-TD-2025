use env_logger::{Builder, Target};
use futures_util::{SinkExt, StreamExt};
use log::{error, info, LevelFilter};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};

async fn handle_connection(stream: TcpStream) {
    let addr = match stream.peer_addr() {
        Ok(addr) => addr,
        Err(e) => {
            error!("Failed to read peer addr: {e}");
            return;
        }
    };

    info!("New connection from {addr}");

    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            error!("WebSocket handshake failed for {addr}: {e}");
            return;
        }
    };

    info!("WebSocket connection established: {addr}");
    let (mut write, mut read) = ws_stream.split();

    // Send welcome message once connected
    if let Err(e) = write
        .send(Message::Text("Welcome to the echo server".into()))
        .await
    {
        error!("Failed to send welcome to {addr}: {e}");
        return;
    }

    // Echo whatever we receive
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                info!("{addr} says: {text}");
                if write.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
            Ok(Message::Close(_)) => {
                info!("Client closed connection: {addr}");
                break;
            }
            Err(e) => {
                error!("WebSocket error for {addr}: {e}");
                break;
            }
            _ => {}
        }
    }

    info!("Connection closed: {addr}");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Builder::new()
        .target(Target::Stdout)
        .filter_level(LevelFilter::Info)
        .init();

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    info!("Echo server listening on ws://127.0.0.1:8080");

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(handle_connection(stream));
    }

    Ok(())
}
