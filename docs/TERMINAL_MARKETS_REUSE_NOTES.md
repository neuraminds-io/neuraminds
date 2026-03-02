# terminal.markets Reuse Notes (Agent-Browser Pass)

Date: 2026-02-26
Source reviewed: https://www.terminal.markets
Method: live browser walkthrough via `agent-browser`

## High-value patterns we should adopt

## 1) Command palette as primary power UI (P0)
Observed:
- `⌘K` opens a command surface with navigation + recent token shortcuts.
- Works as a fast control layer over a dense interface.

Apply in Neuraminds:
- Add `⌘K` command palette with actions:
  - go to Markets, Arena, Leaderboard, Agent Lab
  - open specific market by slug
  - quick actions: create market, place order, claim

## 2) Action-in-context buttons in market rows (P0)
Observed:
- Each token row has direct actions (`Bullish`, `Bearish`, `Analyze`) without leaving list context.
- `Analyze` pre-fills agent chat with a task prompt.

Apply in Neuraminds:
- Per-market inline actions:
  - `Long`, `Short`/`No`, `Analyze`
- `Analyze` should inject a structured prompt into Agent Arena chat for that market.

## 3) Leaderboard with decomposed performance (P0)
Observed:
- Public ranking table with total PnL + realized + unrealized columns.
- Podium treatment for top 3 creates social competition.

Apply in Neuraminds:
- Add ranking for agent vaults with:
  - total PnL
  - realized PnL
  - unrealized PnL
  - win rate
  - trades count

## 4) TokenBook/Entity profile mode (P1)
Observed:
- Alternate mode converts a token into an entity profile:
  - identity, stats, holders, social posts, related entities.

Apply in Neuraminds:
- Add Market Intel view:
  - market metadata + outcome context
  - top participant addresses
  - related markets
  - linked evidence feed (X/news/onchain)

## 5) Persistent live state strip (P1)
Observed:
- Footer continuously shows chain block + action cadence indicator.

Apply in Neuraminds:
- Global status strip:
  - Base latest block
  - indexer lag
  - matcher queue depth
  - last settlement time

## 6) Agent settings mapped to human language (P1)
Observed:
- Sliders map to plain-language summaries (risk/activity/size/holding style).

Apply in Neuraminds:
- Agent configuration panel with 5-7 intent sliders + generated strategy summary.
- Save as signed user strategy profile consumed by agent runtime.

## Patterns to avoid copying

- Hard gate via NFT ownership for core product access (too much onboarding friction for Neuraminds launch).
- Meme-heavy voice for primary trading surface (we should keep brutalist/CLI tone).

## Suggested rollout order

1. Command palette + inline market actions + chat prompt injection.
2. Leaderboard v1 with realized/unrealized PnL split.
3. Market Intel view and global live status strip.
4. Agent settings UX rewrite with natural-language strategy summary.

## Implemented now (ops layer)

- `scripts/dx-terminal-pro.sh` added as DX operator wrapper for reads + optional writes.
- Root `package.json` includes `dx:*` helper scripts for frequent workflows.
- `.env.example` now documents DX env keys.
- Runbook: `docs/runbooks/DX_TERMINAL_OPERATIONS.md`.
- `scripts/launch-readiness.sh` now supports automatic DX snapshot capture during readiness runs.
