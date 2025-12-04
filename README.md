# Rust Async & WebSocket Workshops (TD1 + TD2)

Ce dépôt contient deux ateliers :
- **TD1** : agrégateur de prix boursiers async + PostgreSQL.
- **TD2** : diffusion temps réel via WebSocket et dashboard navigateur.

## Pré-requis communs
- Rust (edition 2021), Cargo.
- PostgreSQL en local (`stockdb`).
- Variables d’environnement (`td01-basics/.env` recommandé) :
  ```
  ALPHA_VANTAGE_API_KEY=...
  FINNHUB_API_KEY=...
  DATABASE_URL=postgresql://<user>@localhost/stockdb
  ```
- Schéma DB : `psql stockdb < td01-basics/schema.sql`.

## TD1 – Exos binaires
Chemin : `td01-basics/src/bin`.

1. **exo1** (bases async)  
   `cargo run --bin exo1`  
   Observe le temps total pour 3 fetchs simulés.

2. **exo2** (APIs)  
   `cargo run --bin exo2`  
   Requiert les clés API, affiche deux sources en parallèle.

3. **exo3** (DB)  
   `cargo run --bin exo3`  
   Remplit `stock_prices` pour AAPL/GOOGL/MSFT (Alpha Vantage + Finnhub) et affiche les dernières lignes.

4. **exo4** (prod-like)  
   `cargo run --bin exo4`  
   Boucle périodique (60s), select! avec Ctrl+C, logging `tracing`. Vérifie que la DB se remplit dans le temps.

Vérifications rapides TD1 :
- `psql stockdb -c "select symbol, price, source, timestamp from stock_prices order by id desc limit 5;"`.
- Traces dans la console (`[OK] Saved ...` pour exo3, logs `tracing` pour exo4).

## TD2 – WebSocket + Dashboard
Crate : `td02-websocket`.

Bins :
- `ws_echo` (partie 1) : echo + message de bienvenue.  
  `cargo run -p td02-websocket --bin ws_echo` → tester avec l’HTML de test ou un client WebSocket sur `ws://127.0.0.1:8080`.

- `ws_broadcast` (partie 2) : simulateur de prix aléatoires, diffusion `broadcast`, `/stats`, compteur de connexions.  
  `cargo run -p td02-websocket --bin ws_broadcast` → clients sur `ws://127.0.0.1:8081`.

- `ws_dashboard` (partie 3) : lit la base `stock_prices` (poll 5s), envoie dernière valeur par symbole/source, répond `/stats`.  
  `cargo run -p td02-websocket --bin ws_dashboard` → clients sur `ws://127.0.0.1:8082`. Charge l’env `.env` ou `td01-basics/.env`.

Dashboard : `td02-websocket/dashboard.html`
- Ouvrir le fichier localement (double-clic ou `python -m http.server 8000` puis `http://127.0.0.1:8000/td02-websocket/dashboard.html`).
- Nécessite des données dans `stock_prices` (lancer exo4 ou insérer une ligne manuelle).

Vérifications rapides TD2 :
- Connexion WebSocket : un message `{"type":"connected",...}` doit arriver.
- Insertion manuelle doit déclencher un push :
  ```
  psql stockdb -c "insert into stock_prices(symbol,price,source,timestamp) values ('TEST',199.99,'finnhub',extract(epoch from now())::bigint);"
  ```
  Le client doit recevoir `{"symbol":"TEST",...}`. (Pensez à supprimer ensuite : `delete from stock_prices where symbol='TEST';`)
- `/stats` côté client doit renvoyer le compteur de connexions.

## Notes
- `ws_dashboard` s’appuie sur le schéma TD1 (`price` en FLOAT4). Pas de LISTEN/NOTIFY : polling 5s.
- Ports utilisés : 8080 (echo), 8081 (broadcast simulateur), 8082 (dashboard DB).
- Pour tester les WebSockets en CLI, installer `websocat` ou utiliser `python -m websockets`.
