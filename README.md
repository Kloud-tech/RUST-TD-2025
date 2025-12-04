# Run Guide

## TD1 (td01-basics)
- `cargo run --bin exo1` : bases async.
- `cargo run --bin exo2` : appels API (clés dans `.env`).
- `cargo run --bin exo3` : écrit dans PostgreSQL (`DATABASE_URL`, schéma importé).
- `cargo run --bin exo4` : boucle 60s avec logs et arrêt Ctrl+C.
Vérif DB : `psql stockdb -c "select symbol, price, source, timestamp from stock_prices order by id desc limit 5;"`.
- Lancer les 4 exos en parallèle (attention aux quotas API) :  
  `bash -c 'cargo run --bin exo1 & cargo run --bin exo2 & cargo run --bin exo3 & cargo run --bin exo4 & wait'`

## TD2 WebSocket (td02-websocket)
- Echo : `cargo run -p td02-websocket --bin ws_echo` → `ws://127.0.0.1:8080`.
- Broadcast simulateur : `cargo run -p td02-websocket --bin ws_broadcast` → `ws://127.0.0.1:8081`.
- Dashboard DB : `cargo run -p td02-websocket --bin ws_dashboard` (charge `.env` ou `td01-basics/.env` avec `DATABASE_URL`) → clients sur `ws://127.0.0.1:8082`.
- UI : ouvrir `td02-websocket/dashboard.html` (double-clic ou `python -m http.server 8000` puis `http://127.0.0.1:8000/td02-websocket/dashboard.html`). Assure-toi que la table `stock_prices` a des données (via exo4 ou insertion).
- `/stats` côté WebSocket renvoie le compteur de connexions.
- Remplissage rapide : `cargo run -p td02-websocket --bin seed_demo` (insère quelques valeurs aléatoires AAPL/GOOGL/MSFT pour les deux sources).
- Remplissage continu (rafraîchissement régulier du dashboard) : `cargo run -p td02-websocket --bin seed_stream` (insère en boucle toutes les quelques secondes ; change la période avec `SEED_PERIOD_SECS=2` par ex.).

## Loglyzer (outil bonus)
- Exemple fourni : `cargo run -p loglyzer -- sample.log`.
- Filtre temporel : `cargo run -p loglyzer -- sample.log --since "2024-01-15 10:00" --until "2024-01-15 12:00"`.
- Suivi live : `cargo run -p loglyzer -- sample.log --follow`, puis ajoute une ligne :  
  `echo '8.8.8.8 - - [15/Jan/2024:12:05:00 +0000] "GET /health HTTP/1.1" 204 0' >> sample.log`.
- Export + serveur JSON : `cargo run -p loglyzer -- sample.log --export-html report.html --serve 9000` (JSON sur `http://localhost:9000/data`, report.html écrit localement).
- Glob multi-fichiers : `cargo run -p loglyzer -- "logs/*.log"`.
- Regex custom (groupes `ip`, `url`, `status`, `time`) : `--pattern "<regex>"`.
- Config optionnelle : `.loglyzer.toml` ou `--config path` (CLI prioritaire).
