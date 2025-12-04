use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use env_logger::Target;
use futures_util::{SinkExt, StreamExt};
use log::{error, info, warn, LevelFilter};
use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio::time::{interval, Duration};
use tokio_tungstenite::{accept_async, tungstenite::Message};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PriceUpdate {
    symbol: String,
    price: f64,
    source: String,
    timestamp: i64,
}

async fn handle_client(
    stream: TcpStream,
    mut rx: broadcast::Receiver<PriceUpdate>,
    connection_count: Arc<AtomicUsize>,
) {
    let addr = match stream.peer_addr() {
        Ok(addr) => addr,
        Err(e) => {
            error!("Failed to read peer addr: {e}");
            return;
        }
    };

    let current = connection_count.fetch_add(1, Ordering::SeqCst) + 1;
    info!("Client connected: {addr} (active: {current})");

    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            error!("WebSocket handshake failed for {addr}: {e}");
            connection_count.fetch_sub(1, Ordering::SeqCst);
            return;
        }
    };

    let (mut write, mut read) = ws_stream.split();

    let welcome = serde_json::json!({
        "type": "connected",
        "message": "Connected to stock price feed"
    });
    if write
        .send(Message::Text(welcome.to_string().into()))
        .await
        .is_err()
    {
        connection_count.fetch_sub(1, Ordering::SeqCst);
        return;
    }

    loop {
        tokio::select! {
            Ok(price_update) = rx.recv() => {
                let json = match serde_json::to_string(&price_update) {
                    Ok(j) => j,
                    Err(e) => {
                        error!("Failed to serialize price update: {e}");
                        continue;
                    }
                };

                if write.send(Message::Text(json)).await.is_err() {
                    info!("Client disconnected while sending: {addr}");
                    break;
                }
            }

            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        info!("Received from {addr}: {text}");
                        if text.trim() == "/stats" {
                            let count = connection_count.load(Ordering::SeqCst);
                            let stats = serde_json::json!({
                                "type": "stats",
                                "active_connections": count
                            });
                            if write.send(Message::Text(stats.to_string())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        info!("Client closed connection: {addr}");
                        break;
                    }
                    Some(Err(e)) => {
                        warn!("WebSocket error for {addr}: {e}");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    let remaining = connection_count.fetch_sub(1, Ordering::SeqCst) - 1;
    info!("Client disconnected: {addr} (active: {remaining})");
}

async fn price_simulator(tx: broadcast::Sender<PriceUpdate>) {
    let mut ticker = interval(Duration::from_secs(2));
    let symbols = vec!["AAPL", "GOOGL", "MSFT"];
    let sources = vec!["alpha_vantage", "finnhub"];

    loop {
        ticker.tick().await;

        // Create RNG per tick to avoid holding non-Send state across awaits
        let mut rng = rand::thread_rng();
        let symbol = symbols[rng.gen_range(0..symbols.len())];
        let source = sources[rng.gen_range(0..sources.len())];
        let price = rng.gen_range(100.0..200.0);

        let update = PriceUpdate {
            symbol: symbol.to_string(),
            price,
            source: source.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        };

        info!("Broadcasting {symbol} @ ${price:.2} from {source}");
        let _ = tx.send(update);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::new()
        .target(Target::Stdout)
        .filter_level(LevelFilter::Info)
        .init();

    let (tx, _rx) = broadcast::channel::<PriceUpdate>(100);
    let connection_count = Arc::new(AtomicUsize::new(0));

    // Spawn simulator
    tokio::spawn(price_simulator(tx.clone()));

    // Start WebSocket server
    let listener = TcpListener::bind("127.0.0.1:8081").await?;
    info!("Broadcast server listening on ws://127.0.0.1:8081");

    while let Ok((stream, _)) = listener.accept().await {
        let rx = tx.subscribe();
        let count = connection_count.clone();
        tokio::spawn(handle_client(stream, rx, count));
    }

    Ok(())
}
