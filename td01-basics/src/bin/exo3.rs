/* Part3
  * Modify the code to save the price in database
  * Fetch prices from 2 APIs for 3 stocks and save all 6 results to the database
  * Query the database to verify the data was saved

---*/
use dotenv::dotenv;
use reqwest;
use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::env;

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

    Ok(())
}

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv().ok();

    println!("Stock Price Aggregator with PostgreSQL\n");

    // Connect to database
    let database_url = env::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("Connected to database\n");

    // Stocks to fetch
    let symbols = vec!["AAPL", "GOOGL", "MSFT"];

    // Fetch and save prices for all stocks
    for symbol in &symbols {
        println!("Fetching prices for {}...", symbol);

        // Fetch from both APIs in parallel
        let (alpha_result, finnhub_result) =
            tokio::join!(fetch_alpha_vantage(symbol), fetch_finnhub(symbol));

        // Save Alpha Vantage result
        match alpha_result {
            Ok(price) => {
                match save_price(&pool, &price).await {
                    Ok(_) => println!("  [OK] Saved Alpha Vantage: ${:.2}", price.price),
                    Err(e) => println!("  [ERROR] Failed to save Alpha Vantage: {}", e),
                }
            }
            Err(e) => println!("  [WARN] Alpha Vantage error: {}", e),
        }

        // Save Finnhub result
        match finnhub_result {
            Ok(price) => {
                match save_price(&pool, &price).await {
                    Ok(_) => println!("  [OK] Saved Finnhub: ${:.2}", price.price),
                    Err(e) => println!("  [ERROR] Failed to save Finnhub: {}", e),
                }
            }
            Err(e) => println!("  [WARN] Finnhub error: {}", e),
        }

        println!();
    }

    // Query and display saved data
    println!("\nRecent stock prices from database:\n");

    let rows = sqlx::query!(
        r#"
        SELECT symbol, price, source, timestamp
        FROM stock_prices
        ORDER BY id DESC
        LIMIT 10
        "#
    )
    .fetch_all(&pool)
    .await?;

    for row in rows {
        let dt = chrono::DateTime::from_timestamp(row.timestamp, 0)
            .unwrap()
            .format("%Y-%m-%d %H:%M:%S");
        println!(
            "  {} | ${:>8.2} | {:>15} | {}",
            row.symbol, row.price, row.source, dt
        );
    }

    println!("\nDone!");

    Ok(())
}
