# Run Guide

Prérequis : `.env` (ou `td01-basics/.env`) avec `ALPHA_VANTAGE_API_KEY`, `FINNHUB_API_KEY`, `DATABASE_URL`, base `stockdb` avec le schéma (`td01-basics/schema.sql`).

Lancer rapidement:

- Broadcast simulateur : `cargo run -p td02-websocket --bin ws_broadcast` (ws://127.0.0.1:8081)
- Dashboard DB : `cargo run -p td02-websocket --bin ws_dashboard` (ws://127.0.0.1:8082)
- Front : ouvrir `td02-websocket/dashboard.html` (double-clic ou `python -m http.server 8000` puis `http://127.0.0.1:8000/td02-websocket/dashboard.html`)
-- Donnée API  : `cargo run --bin exo4`

## TD1 (td01-basics)

**Ne pas run pour le projet:**

- `cargo run --bin exo1` (bases async)
- `cargo run --bin exo2` (APIs)
- `cargo run --bin exo3` (écrit en DB)
- `cargo run --bin exo4` (boucle 60s, logs, Ctrl+C)
  Vérif DB : `psql stockdb -c "select symbol, price, source, timestamp from stock_prices order by id desc limit 5;"`.

## TD2 WebSocket (td02-websocket)

- Echo : `cargo run -p td02-websocket --bin ws_echo` (WebSocket sur ws://127.0.0.1:8080)
- Broadcast simulateur : `cargo run -p td02-websocket --bin ws_broadcast` (ws://127.0.0.1:8081)
- Dashboard DB : `cargo run -p td02-websocket --bin ws_dashboard` (ws://127.0.0.1:8082)
- Front : ouvrir `td02-websocket/dashboard.html` (double-clic ou `python -m http.server 8000` puis `http://127.0.0.1:8000/td02-websocket/dashboard.html`)
-- Donnée API  : `cargo run --bin exo4`
- ***Données demo si API pas disponible:** *`cargo run -p td02-websocket --bin seed_demo` (shot) ou `cargo run -p td02-websocket --bin seed_stream` (en continu, ajustable avec `SEED_PERIOD_SECS=2`)


## Loglyzer (bonus)

- `cargo run -p loglyzer -- sample.log` (exemple fourni)
- Suivi live : `cargo run -p loglyzer -- sample.log --follow` puis ajouter une ligne : `echo '8.8.8.8 - - [15/Jan/2024:12:05:00 +0000] "GET /health HTTP/1.1" 204 0' >> sample.log`
