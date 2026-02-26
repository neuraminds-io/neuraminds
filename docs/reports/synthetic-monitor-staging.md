# Synthetic Monitor Report

Environment: staging
Generated: 2026-02-26T18:38:59.208Z
API: https://neuraminds-web-base-staging-v4.onrender.com
Web: https://neuraminds-web-base-staging-v4.onrender.com
Chain mode: base

Decision: PASS

## Checks
- api_health: PASS (291ms) status=healthy
- api_health_detailed: PASS (587ms) http=200 db=unknown redis=unknown solana=unknown base=healthy mode=base
- api_evm_markets_public: PASS (275ms) marketCount=1
- api_evm_orderbook_smoke: PASS (213ms) bids=0 asks=0
- api_evm_trades_smoke: PASS (329ms) tradeCount=0
- web_home: PASS (233ms) http=200 contentType=text/html; charset=utf-8

## Table
| Check | Status | Latency (ms) | URL | Details |
| --- | --- | ---: | --- | --- |
| api_health | PASS | 291 | https://neuraminds-web-base-staging-v4.onrender.com/health | status=healthy |
| api_health_detailed | PASS | 587 | https://neuraminds-web-base-staging-v4.onrender.com/health/detailed | http=200 db=unknown redis=unknown solana=unknown base=healthy mode=base |
| api_evm_markets_public | PASS | 275 | https://neuraminds-web-base-staging-v4.onrender.com/v1/evm/markets?limit=1 | marketCount=1 |
| api_evm_orderbook_smoke | PASS | 213 | https://neuraminds-web-base-staging-v4.onrender.com/v1/evm/markets/1/orderbook?outcome=yes&depth=5 | bids=0 asks=0 |
| api_evm_trades_smoke | PASS | 329 | https://neuraminds-web-base-staging-v4.onrender.com/v1/evm/markets/1/trades?limit=1 | tradeCount=0 |
| web_home | PASS | 233 | https://neuraminds-web-base-staging-v4.onrender.com | http=200 contentType=text/html; charset=utf-8 |

