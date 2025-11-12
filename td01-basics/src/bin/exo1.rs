/*### **Part 1 – Intro to Async & Tokio Runtime (30 min)**

**Building Block**: Understanding async foundations

* **Concepts**:
  * What is async/await in Rust? (Future-based execution, cooperative multitasking)
  * Why we need a runtime like Tokio
  * Difference between sync vs async IO
  * How this applies to fetching stock prices

* **Demo Code**: Minimal async example with `tokio::main`

```rust
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    println!("Starting stock price simulator...");

    let fetch_price = async {
        sleep(Duration::from_millis(500)).await;
        println!("AAPL: $150.25");
    };

    fetch_price.await;
}
```

* **Exercise**:
  * Create an async function `fetch_mock_price(symbol: &str) -> f64` that sleeps for 500ms and returns a random price
  * Call it for 3 different stock symbols sequentially
  * Observe the total time taken

> When installing tokio, enable all features: `tokio = { version = "1.47.1", features = ["full"] }`
---*/
use tokio::time::{sleep, Duration};
use rand::Rng;

#[tokio::main]
async fn main(){
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


