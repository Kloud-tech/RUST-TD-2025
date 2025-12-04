/* Part 4
  * Use `tokio::time::interval` for periodic tasks (fetch every minute)
  * Use `tokio::select!` for handling multiple concurrent operations
  * Implement graceful shutdown with `tokio::signal::ctrl_c()`
  * Add structured logging with `tracing`

---*/
use dotenv;
use reqwest;
use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::env;
use tokio::signal;
use tokio::time::{interval, Duration};
use tracing::{error, info, instrument, warn};

#[derive(Deserialize, Debug)]
struct GlobalQuote {
    #[serde(rename = "Global Quote")]
    quote: Quote,
}

#[derive(Deserialize, Debug)]
struct Quote {
    #[serde(rename = "01. symbol")]
    symbol: String,
    #[serde(rename = "05. price")]
    price: String,
}

#[derive(Deserialize, Debug)]
struct AlphaVantageError {
    #[serde(rename = "Information")]
    information: Option<String>,
    #[serde(rename = "Error Message")]
    error_message: Option<String>,
}

#[derive(Deserialize, Debug)]
struct FinnhubQuote {
    c: f64, // current price
}

#[derive(Debug, Clone)]
struct StockPrice {
    symbol: String,
    price: f64,
    source: String,
    timestamp: i64,
}

#[instrument(skip(pool))]
async fn save_price(pool: &PgPool, price: &StockPrice) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO stock_prices (symbol, price, source, timestamp)
        VALUES ($1, $2, $3, $4)
        "#,
        price.symbol,
        price.price as f32,
        price.source,
        price.timestamp
    )
    .execute(pool)
    .await?;

    info!(
        symbol = %price.symbol,
        price = %price.price,
        source = %price.source,
        "Saved price to database"
    );

    Ok(())
}

#[instrument]
async fn fetch_alpha_vantage(symbol: &str) -> Result<StockPrice, Box<dyn std::error::Error>> {
    let api_key = env::var("ALPHA_VANTAGE_API_KEY")?;
    let url = format!(
        "https://www.alphavantage.co/query?function=GLOBAL_QUOTE&symbol={}&apikey={}",
        symbol, api_key
    );

    let text = reqwest::get(&url).await?.text().await?;

    // Check for rate limit or error message
    if let Ok(error) = serde_json::from_str::<AlphaVantageError>(&text) {
        if let Some(info) = error.information {
            return Err(format!("Rate limit: {}", info).into());
        }
        if let Some(msg) = error.error_message {
            return Err(format!("API error: {}", msg).into());
        }
    }

    let resp: GlobalQuote = serde_json::from_str(&text)?;
    let price: f64 = resp.quote.price.parse()?;

    Ok(StockPrice {
        symbol: symbol.to_string(),
        price,
        source: "alpha_vantage".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

#[instrument]
async fn fetch_finnhub(symbol: &str) -> Result<StockPrice, Box<dyn std::error::Error>> {
    let api_key = env::var("FINNHUB_API_KEY")?;
    let url = format!(
        "https://finnhub.io/api/v1/quote?symbol={}&token={}",
        symbol, api_key
    );

    let resp = reqwest::get(&url).await?.json::<FinnhubQuote>().await?;

    Ok(StockPrice {
        symbol: symbol.to_string(),
        price: resp.c,
        source: "finnhub".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

#[instrument(skip(pool))]
async fn fetch_and_save_all(
    pool: &PgPool,
    symbols: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting fetch cycle for {} symbols", symbols.len());

    for symbol in symbols {
        // Fetch from multiple sources
        let (alpha_result, finnhub_result) =
            tokio::join!(fetch_alpha_vantage(symbol), fetch_finnhub(symbol));

        // Save results
        if let Ok(price) = alpha_result {
            if let Err(e) = save_price(pool, &price).await {
                error!(symbol = %symbol, error = %e, "Failed to save alpha_vantage price");
            }
        } else if let Err(e) = alpha_result {
            warn!(symbol = %symbol, error = %e, "Failed to fetch from alpha_vantage");
        }

        if let Ok(price) = finnhub_result {
            if let Err(e) = save_price(pool, &price).await {
                error!(symbol = %symbol, error = %e, "Failed to save finnhub price");
            }
        } else if let Err(e) = finnhub_result {
            warn!(symbol = %symbol, error = %e, "Failed to fetch from finnhub");
        }
    }

    info!("Completed fetch cycle");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file in current directory or parent
    dotenv::from_filename(".env")
        .ok()
        .or_else(|| dotenv::from_filename("td01-basics/.env").ok());

    // Setup tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .init();

    info!("Starting stock price aggregator");

    // Configuration
    let symbols = vec!["AAPL".to_string(), "GOOGL".to_string(), "MSFT".to_string()];

    // Setup database connection pool
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env file");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    info!("Connected to database");

    // Create interval for periodic fetching (every 60 seconds)
    let mut fetch_interval = interval(Duration::from_secs(60));

    info!("Starting periodic fetch loop (every 60 seconds). Press Ctrl+C to stop.");

    // Main loop
    loop {
        tokio::select! {
            _ = fetch_interval.tick() => {
                if let Err(e) = fetch_and_save_all(&pool, &symbols).await {
                    error!(error = %e, "Error during fetch cycle");
                }
            }
            _ = signal::ctrl_c() => {
                info!("Shutdown signal received");
                break;
            }
        }
    }

    // Graceful shutdown
    info!("Closing database connections...");
    pool.close().await;
    info!("Shutdown complete");

    Ok(())
}
