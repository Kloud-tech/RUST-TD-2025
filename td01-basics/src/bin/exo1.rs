/* Part 1
  * Create an async function `fetch_mock_price(symbol: &str) -> f64` that sleeps for 500ms and returns a random price
  * Call it for 3 different stock symbols sequentially
  * Observe the total time taken

---*/
use rand::Rng;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    println!("Starting stock price simulator...");

    let start = std::time::Instant::now();

    let price1 = fetch_mock_price("AAPL").await;
    println!("AAPL: ${:.2}", price1);

    let price2 = fetch_mock_price("GOOG").await;
    println!("GOOG: ${:.2}", price2);

    let price3 = fetch_mock_price("MSFT").await;
    println!("MSFT: ${:.2}", price3);

    let elapsed = start.elapsed();
    println!("Total time taken: {:.2}s", elapsed.as_secs_f64());
}

async fn fetch_mock_price(_symbol: &str) -> f64 {
    sleep(Duration::from_millis(500)).await;
    rand::thread_rng().gen_range(50.0..500.0)
}
