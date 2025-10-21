# Percolator • Perp Sim

![Perp Sim Banner](banner.png)

A fast, self-contained **perp-style dashboard** with **live BTC/ETH prices**, a **synthetic order book**, and a **paper-trading engine**. It looks and feels like a CEX/DEX perp UI, without needing on-chain programs. Perfect for prototyping Solana perp UX (router/slab) while instruction handlers are still evolving.

> **Status**: Simulation only. No real orders are sent to any exchange or chain.

---

## Table of Contents

- [What You Get](#what-you-get)
- [Live Demo & Screens](#live-demo--screens)
- [Fork / Clone](#fork--clone)
- [Requirements](#requirements)
- [Run Locally](#run-locally)
  - [Windows PowerShell](#windows-powershell)
  - [macOS / Linux / WSL](#macos--linux--wsl)
- [Deploy to Render](#deploy-to-render)
- [Project Layout](#project-layout)
- [Configuration](#configuration)
- [API](#api)
- [Solana Local RPC (Optional Notes)](#solana-local-rpc-optional-notes)
- [Troubleshooting](#troubleshooting)
- [Roadmap](#roadmap)
- [Security & Disclaimers](#security--disclaimers)
- [License](#license)
- [Credits](#credits)

---

## What You Get

- **Live prices** (Binance public API) for `BTCUSDT` and `ETHUSDT`
- **1-minute candles** rendered with Lightweight Charts
- **Synthetic order book** updated every ~1.5s with **mid** & **BBO**
- **Paper-trading engine** (market orders)
  - Cash, Equity, Used Margin, Margin Ratio
  - Position uPnL and **estimated liquidation price**
- **Trades tape** (your fills + small “noise” prints so the tape is alive)
- **Realtime updates** via **SSE** (Server-Sent Events)
- **Deploy-ready** on Render (free tier)

> The code is tiny and hackable—great for demos, hackathons, and educational use.

---

## Live Demo & Screens

When you run locally or deploy on Render, you’ll see:

- Markets sidebar (BTC 100x / ETH 100x)
- Candlestick chart (1m) with a dark theme
- Synthetic order book (bids/asks, best bid/ask, mid)
- Trades tape (side/price/qty/time), auto-updating
- Order form (market orders), and position metrics

---

## Fork / Clone

**Fork** this repo on GitHub, or clone it directly:

```bash
git clone https://github.com/<your-username>/percolator-perp-sim.git
cd percolator-perp-sim
```

If you cloned our original template and want to make it yours:

```bash
# rename origin to upstream, then point origin to your new repo
git remote rename origin upstream
git remote add origin https://github.com/<your-username>/percolator-perp-sim.git
git push -u origin main
```

> Git LFS is **not** required. The only asset is `banner.png` (~<1MB).

---

## Requirements

- **Node.js 20+** (global `fetch` used by the server)
- **npm** (bundled with Node)
- Internet access (to fetch Binance tickers/klines CDN & chart library)

> No database, no Redis, no extra services.

---

## Run Locally

### Windows PowerShell

```powershell
# navigate to the folder (example)
Set-Location "C:\Users\you\projects\percolator-perp-sim"

# install deps
npm install

# run
node .\server.js

# open browser
# http://localhost:3000
```

### macOS / Linux / WSL

```bash
cd percolator-perp-sim
npm install
node server.js
# open http://localhost:3000
```

> If the chart is blank, make sure the page loaded Lightweight-Charts from the CDN and the `#tv` container has height (this repo sets it to 420px).

---

## Deploy to Render

1. Push this folder to **GitHub**.
2. On **Render**: **New → Web Service**.
3. Settings:
   - **Runtime**: Node
   - **Build Command**: `npm install`
   - **Start Command**: `node server.js`
   - **Instance Type**: Free
   - Region: closest to you
4. Click **Create Web Service** and wait for the build.

The app listens on `process.env.PORT` automatically.

---

## Project Layout

```
.
├─ public/
│  └─ index.html          # UI: chart, orderbook, tape, order form
├─ server.js              # Express server: market data, SSE, sim logic
├─ package.json
└─ banner.png             # GitHub README banner
```

---

## Configuration

Tweak constants in **`server.js`**:

```js
// symbols served by the app
const symbols = ["BTCUSDT", "ETHUSDT"];

// candle lookback (1m interval)
const CANDLE_LIMIT = 500;

// synthetic orderbook density (inside synthOrderBook())
// - tick size and baseQty differ per symbol
```

Other knobs:

- **Starting balance**: `state.cash = 10_000` (USDT)
- **Leverage/IM/MM**: `leverage = 10`, `im = 0.1`, `mm = 0.05`
- **Noise trades** interval: ~`1200ms` (keeps the tape alive)

Add more markets by appending to `symbols` and adding matching UI labels in `index.html`.

---

## API

- `GET /events` → **SSE** stream:
  - `{"type":"snapshot","data":{...}}` every second
  - `{"type":"trade","data":{...}}` on new trades (noise or your fills)
- `GET /api/tickers` → `{ BTCUSDT: {price, ts}, ETHUSDT: {…} }`
- `GET /api/candles?symbol=BTCUSDT` → 1m klines (limit 500)
- `GET /api/orderbook?symbol=BTCUSDT` → synthetic bids/asks + `mid`
- `GET /api/trades?symbol=BTCUSDT` → recent trades (newest first)
- `GET /api/portfolio` → `{ cash, equity, usedMargin, marginRatio, positions }`
- `POST /api/order` (market-only):
  ```json
  { "symbol": "BTCUSDT", "side": "buy", "qty": 0.001 }
  ```

> This is a **demo API** (no auth, no persistence). Don’t expose secrets.

---

## Solana Local RPC (Optional Notes)

This UI does **not** require a Solana node. If you’re testing Solana in parallel (e.g., prepping router/slab programs), here’s a quick reference:

**Start a local validator (Windows):**
```powershell
solana-test-validator --bind-address 0.0.0.0 --rpc-port 8899 --reset
```

**Point CLI to localnet & create a wallet:**
```powershell
solana config set --url http://127.0.0.1:8899
solana-keygen new -o wallet-keypair.json
solana airdrop 10
solana balance
```

**Notes**
- Devnet funds ≠ Localnet funds. Use `solana airdrop` on localnet only.
- Rate-limited airdrop? Wait a few seconds and retry.
- This sim pulls **off-chain prices** and works fine without any RPC.

---

## Troubleshooting

**Chart is blank**
- Ensure CDN access to Lightweight-Charts.
- `#tv` must have a non-zero height (we set it to 420px).

**`Cannot find module 'express'`**
- Run `npm install` in the repo folder. Commit `package.json` & lockfile before deploying.

**Render 502 / port mismatch**
- Don’t hardcode a port. The app uses `process.env.PORT || 3000`.

**Ticker/candle fetch errors**
- Temporary Binance throttle—try again, or increase polling intervals.

**Windows path issues**
- Use PowerShell in the repo folder:
  `Set-Location "C:\Users\you\projects\percolator-perp-sim"`.

---

## Roadmap

- Limit orders & in-memory matching
- Funding-rate estimator
- Persistence (Redis/Postgres) for multi-user demos
- Wallet Connect (Solana/EVM) for UI parity
- WebSocket transport option (SSE is ideal for free-tier hosting)

---

## Security & Disclaimers

- For **educational/demo** use only. No financial advice.
- Never paste secrets/private keys anywhere in the app or repo.
- The server is intentionally **stateless** and unauthenticated.

---

## License

MIT — do whatever; attribution appreciated.

---

## Credits

- **Lightweight Charts** (TradingView) for the chart renderer
- **Express** for the web server
- **Binance public REST** for tickers and 1m klines
- Perp design inspiration from the router/slab architecture explored on Solana
