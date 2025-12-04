use chrono::Utc;
use dotenvy::dotenv;
use rand::Rng;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    dotenvy::from_filename("td01-basics/.env").ok();

    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env or environment");
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&database_url)
        .await?;

    let symbols = ["AAPL", "GOOGL", "MSFT"];
    let sources = ["alpha_vantage", "finnhub"];
    let mut rng = rand::thread_rng();
    let now = Utc::now().timestamp();

    for symbol in symbols {
        for source in sources {
            let price: f32 = rng.gen_range(120.0..220.0) as f32;
            sqlx::query!(
                r#"
                INSERT INTO stock_prices (symbol, price, source, timestamp)
                VALUES ($1, $2, $3, $4)
                "#,
                symbol,
                price,
                source,
                now
            )
            .execute(&pool)
            .await?;
            println!("Seeded {symbol} from {source} at ${price:.2}");
        }
    }

    println!("Done seeding demo data.");
    Ok(())
}
