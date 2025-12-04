# 🦀 Rust Async Workshop - Stock Price Aggregator

A production-ready asynchronous stock price aggregator built with Rust, demonstrating advanced async programming patterns, concurrent API calls, database persistence, and graceful shutdown handling.

## 📋 Project Overview

This project implements a complete stock price aggregation system that:

- Fetches real-time stock prices from multiple APIs in parallel
- Stores historical price data in PostgreSQL
- Runs periodic updates with configurable intervals
- Provides structured logging for monitoring and debugging
- Handles graceful shutdown with proper resource cleanup

## 🏗️ Architecture

The project is structured in 4 progressive exercises, each building upon the previous:

### Part 1: Async Foundations (`exo1.rs`)

- Introduction to async/await patterns in Rust
- Mock price fetching with `tokio::time::sleep`
- Sequential vs parallel execution comparison
- Understanding the Tokio runtime

### Part 2: API Integration (`exo2.rs`)

- HTTP requests with `reqwest` (async)
- JSON parsing with `serde`
- Parallel API calls using `tokio::join!`
- Error handling with `Result<T, E>`
- Integration with:
  - Alpha Vantage API (financial data)
  - Finnhub API (real-time quotes)

### Part 3: Database Persistence (`exo3.rs`)

- PostgreSQL integration with `sqlx`
- Async database operations
- Connection pooling
- Batch processing for multiple stocks
- Data verification and retrieval

### Part 4: Production-Ready Application (`exo4.rs`)

- Periodic task execution with `tokio::time::interval`
- Concurrent operation handling with `tokio::select!`
- Signal handling for graceful shutdown (`Ctrl+C`)
- Structured logging with `tracing`
- Resource cleanup and error recovery

## 🚀 Features

- **Asynchronous Architecture**: Leverages Tokio for efficient concurrent operations
- **Multi-Source Data**: Fetches from multiple APIs simultaneously
- **Reliable Storage**: PostgreSQL with connection pooling
- **Production Logging**: Structured logs with tracing for observability
- **Graceful Shutdown**: Proper cleanup of database connections and resources
- **Error Resilience**: Continues operating even if individual API calls fail
- **Type Safety**: Compile-time guarantees with Rust's type system

## 🛠️ Tech Stack

- **Language**: Rust (Edition 2021)
- **Runtime**: Tokio (async runtime)
- **HTTP Client**: reqwest (with JSON support)
- **Database**: PostgreSQL with SQLx (async driver)
- **Serialization**: serde + serde_json
- **Logging**: tracing + tracing-subscriber
- **Environment**: dotenv for configuration

## 📦 Installation

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install PostgreSQL (macOS)
brew install postgresql@15
brew services start postgresql@15

# Create database
createdb stockdb
```

### Setup

1. Clone the repository:

```bash
git clone https://github.com/Kloud-tech/RUST-TD-2025.git
cd RUST-TD-2025/td01-basics
```

2. Create the database schema:

```bash
psql stockdb < schema.sql
```

3. Configure environment variables (`.env`):

```env
ALPHA_VANTAGE_API_KEY=your_key_here
FINNHUB_API_KEY=your_key_here
DATABASE_URL=postgresql://username@localhost/stockdb
```

4. Install dependencies:

```bash
cargo build
```

## 🎯 Usage

### Run Individual Exercises

```bash
# Part 1: Async basics
cargo run --bin exo1

# Part 2: API integration
cargo run --bin exo2

# Part 3: Database persistence
cargo run --bin exo3

# Part 4: Production application
cargo run --bin exo4
```

### Production Application (exo4)

The main application runs continuously, fetching prices every 60 seconds:

```bash
export DATABASE_URL=postgresql://username@localhost/stockdb
cargo run --bin exo4
```

**Output Example:**

```
2025-11-12T15:22:21.520065Z  INFO Starting stock price aggregator
2025-11-12T15:22:21.528484Z  INFO Connected to database
2025-11-12T15:22:21.528513Z  INFO Starting periodic fetch loop (every 60 seconds)
2025-11-12T15:22:21.721336Z  INFO Saved price to database symbol=AAPL price=273.24
2025-11-12T15:22:21.859994Z  INFO Saved price to database symbol=GOOGL price=286.98
2025-11-12T15:22:22.028760Z  INFO Saved price to database symbol=MSFT price=504.90
```

Press `Ctrl+C` for graceful shutdown.

### Query the Database

```bash
# View recent prices
psql stockdb -c "SELECT * FROM stock_prices ORDER BY created_at DESC LIMIT 10;"

# Statistics by symbol
psql stockdb -c "
  SELECT 
    symbol, 
    COUNT(*) as count, 
    ROUND(AVG(price)::numeric, 2) as avg_price,
    ROUND(MIN(price)::numeric, 2) as min_price,
    ROUND(MAX(price)::numeric, 2) as max_price
  FROM stock_prices 
  GROUP BY symbol 
  ORDER BY symbol;
"
```

## 📊 Database Schema

```sql
CREATE TABLE stock_prices (
    id SERIAL PRIMARY KEY,
    symbol VARCHAR(10) NOT NULL,
    price REAL NOT NULL,
    source VARCHAR(50) NOT NULL,
    timestamp BIGINT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_symbol_timestamp ON stock_prices(symbol, timestamp DESC);
```

## 🔧 Configuration

### Environment Variables

| Variable                  | Description                  | Example                                 |
| ------------------------- | ---------------------------- | --------------------------------------- |
| `ALPHA_VANTAGE_API_KEY` | Alpha Vantage API key        | `YOUR_KEY`                            |
| `FINNHUB_API_KEY`       | Finnhub API key              | `YOUR_KEY`                            |
| `DATABASE_URL`          | PostgreSQL connection string | `postgresql://user@localhost/stockdb` |

### API Keys

- **Alpha Vantage**: https://www.alphavantage.co/support/#api-key (Free tier: 25 requests/day)
- **Finnhub**: https://finnhub.io/register (Free tier: 60 calls/minute)

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run with logging
RUST_LOG=debug cargo test

# Check for errors
cargo check
```

## 📈 Performance Considerations

- **Parallel API Calls**: Uses `tokio::join!` to fetch from multiple sources simultaneously
- **Connection Pooling**: Maintains a pool of 5 database connections for efficiency
- **Non-Blocking I/O**: All I/O operations are async, preventing thread blocking
- **Error Isolation**: Individual API failures don't crash the application

## 🔒 Error Handling

The application implements comprehensive error handling:

- **API Rate Limits**: Detects and logs rate limit errors gracefully
- **Network Failures**: Retries and continues with available data
- **Database Errors**: Logs errors without stopping the fetch cycle
- **Type Safety**: Compile-time checks prevent many runtime errors

## 📚 Learning Outcomes

This project demonstrates:

1. **Async/Await Patterns**: Understanding Rust's async programming model
2. **Concurrent Programming**: Using tokio for parallel operations
3. **Database Integration**: Async SQL with connection pooling
4. **API Integration**: HTTP requests, JSON parsing, error handling
5. **Production Practices**: Logging, graceful shutdown, resource management
6. **Type Safety**: Leveraging Rust's type system for reliability

## 🤝 Contributing

This is a workshop project for educational purposes. Feel free to:

- Fork and experiment with the code
- Add new data sources
- Implement additional features (REST API, web dashboard, alerts)
- Optimize performance

## 📝 License

This project is part of a Rust workshop and is available for educational purposes.

## 👨‍💻 Author

**Alexandre**

- Workshop: Rust Async Programming with Tokio
- Focus: Production-ready async applications with real-world APIs

## 🔗 Resources

- [Tokio Documentation](https://tokio.rs)
- [SQLx Documentation](https://docs.rs/sqlx)
- [Tracing Documentation](https://docs.rs/tracing)
- [Alpha Vantage API](https://www.alphavantage.co/documentation/)
- [Finnhub API](https://finnhub.io/docs/api)

---

**Built with ❤️ using Rust**
