/* Part 2
  * Create a second function `fetch_finnhub(symbol: &str)` for Finnhub API
  * Create a struct `StockPrice { symbol: String, price: f64, source: String, timestamp: i64 }`
  * Fetch the same stock from both APIs in parallel using `tokio::join!`
  * Compare the results

---*/
use dotenv::dotenv;
use reqwest;
use serde::Deserialize;
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
    c: f64,  // current price
    h: f64,  // high
    l: f64,  // low
    o: f64,  // open
    pc: f64, // previous close
    t: i64,  // timestamp
}

#[derive(Debug)]
struct StockPrice {
    symbol: String,
    price: f64,
    source: String,
    timestamp: i64,
}

async fn fetch_alpha_vantage(symbol: &str) -> Result<f64, Box<dyn std::error::Error>> {
    let api_key = env::var("ALPHA_VANTAGE_API_KEY")?;
    let url = format!(
        "https://www.alphavantage.co/query?function=GLOBAL_QUOTE&symbol={}&apikey={}",
        symbol, api_key
    );

    let resp = reqwest::get(&url).await?.json::<GlobalQuote>().await?;

    Ok(resp.quote.price.parse()?)
}

async fn fetch_finnhub(symbol: &str) -> Result<f64, Box<dyn std::error::Error>> {
    let api_key = env::var("FINNHUB_API_KEY")?;
    let url = format!(
        "https://finnhub.io/api/v1/quote?symbol={}&token={}",
        symbol, api_key
    );

    let resp = reqwest::get(&url).await?.json::<FinnhubQuote>().await?;

    Ok(resp.c)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenv().ok();

    let symbol = "AAPL";

    // Fetch from both APIs
    let (alpha_result, finnhub_result) =
        tokio::join!(fetch_alpha_vantage(symbol), fetch_finnhub(symbol));

    // Handle results
    match alpha_result {
        Ok(price) => {
            let stock = StockPrice {
                symbol: symbol.to_string(),
                price,
                source: "Alpha Vantage".to_string(),
                timestamp: chrono::Utc::now().timestamp(),
            };
            println!("{:?}", stock);
        }
        Err(e) => println!("Alpha Vantage error: {}", e),
    }

    match finnhub_result {
        Ok(price) => {
            let stock = StockPrice {
                symbol: symbol.to_string(),
                price,
                source: "Finnhub".to_string(),
                timestamp: chrono::Utc::now().timestamp(),
            };
            println!("{:?}", stock);
        }
        Err(e) => println!("Finnhub error: {}", e),
    }

    Ok(())
}
