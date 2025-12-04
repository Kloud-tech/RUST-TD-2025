use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use env_logger::Target;
use futures_util::{SinkExt, StreamExt};
use log::{error, info, warn, LevelFilter};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::FromRow;
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

#[derive(Debug, FromRow)]
struct PriceRow {
    symbol: String,
    price: f32, // matches FLOAT4 in schema
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
        .send(Message::Text(welcome.to_string()))
        .await
        .is_err()
    {
        connection_count.fetch_sub(1, Ordering::SeqCst);
        return;
    }

    loop {
        tokio::select! {
            Ok(price_update) = rx.recv() => {
                if let Ok(json) = serde_json::to_string(&price_update) {
                    if write.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
            }

            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        info!("Received from {addr}: {text}");
                        if text.trim() == "/stats" {
                            let stats = serde_json::json!({
                                "type": "stats",
                                "active_connections": connection_count.load(Ordering::SeqCst)
                            });
                            let _ = write.send(Message::Text(stats.to_string())).await;
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

async fn poll_database(
    pool: &sqlx::PgPool,
    tx: &broadcast::Sender<PriceUpdate>,
    last_seen: &mut HashMap<(String, String), i64>,
) -> Result<(), sqlx::Error> {
    let prices = sqlx::query_as::<_, PriceRow>(
        r#"
        SELECT DISTINCT ON (symbol, source)
            symbol, price, source, timestamp
        FROM stock_prices
        ORDER BY symbol, source, timestamp DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    for row in prices {
        let key = (row.symbol.clone(), row.source.clone());
        let should_send = last_seen
            .get(&key)
            .map(|ts| *ts < row.timestamp)
            .unwrap_or(true);

        if should_send {
            last_seen.insert(key, row.timestamp);
            let update = PriceUpdate {
                symbol: row.symbol,
                price: row.price as f64,
                source: row.source,
                timestamp: row.timestamp,
            };
            let _ = tx.send(update);
        }
    }

    Ok(())
}

async fn database_poller(pool: sqlx::PgPool, tx: broadcast::Sender<PriceUpdate>) {
    let mut ticker = interval(Duration::from_secs(5));
    let mut last_seen: HashMap<(String, String), i64> = HashMap::new();

    loop {
        ticker.tick().await;

        if let Err(e) = poll_database(&pool, &tx, &mut last_seen).await {
            error!("Database poll error: {e}");
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    dotenvy::from_filename("td01-basics/.env").ok();

    env_logger::Builder::new()
        .target(Target::Stdout)
        .filter_level(LevelFilter::Info)
        .init();

    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env or environment");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    info!("Connected to database");

    let (tx, _rx) = broadcast::channel::<PriceUpdate>(100);
    let connection_count = Arc::new(AtomicUsize::new(0));

    // Spawn DB poller
    tokio::spawn(database_poller(pool.clone(), tx.clone()));

    // Start WebSocket server
    let listener = TcpListener::bind("127.0.0.1:8082").await?;
    info!("Dashboard WebSocket server on ws://127.0.0.1:8082");

    while let Ok((stream, _)) = listener.accept().await {
        let rx = tx.subscribe();
        let count = connection_count.clone();
        tokio::spawn(handle_client(stream, rx, count));
    }

    Ok(())
}
