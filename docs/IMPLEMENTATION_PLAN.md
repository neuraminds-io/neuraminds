# Polyguard Implementation Plan

Technical roadmap for building a production-grade Solana prediction market platform.

## Executive Summary

Based on analysis of OpenBook v2, Manifest, Switchboard v2, and Hedgehog Markets, this document outlines the implementation strategy for Polyguard's next phases:

1. **Phase 1**: Enhanced On-Chain Program (CLOB + Oracle Integration)
2. **Phase 2**: AI Trading Agents
3. **Phase 3**: Social Trading with ZK Privacy
4. **Phase 4**: Prediction-as-a-Service (PaaS)
5. **Phase 5**: DeFi Integration

---

## Current State

**All Phases Complete** (as of 2026-01-22)

### Phase 1: On-Chain CLOB - COMPLETE
- Central Limit Order Book (CLOB) with Red-Black tree (512 orders per side)
- Event heap for settlement (256 events)
- Open orders account with position tracking (24 orders per user)
- Full order matching engine (Limit, Market, PostOnly, IOC, FOK)
- Switchboard oracle integration for resolution
- Market resolution (oracle-based and manual)
- Settlement and redemption (mint/burn token sets)
- Event processing crank (consume events, prune orders)

### Phase 2: AI Trading Agents - COMPLETE
- TradingAgent on-chain account with risk parameters
- Position sizing strategies (Fixed, Kelly, Proportional)
- Risk checks (max position, drawdown, daily loss)
- Agent SDK (TypeScript): MomentumStrategy, MeanReversionStrategy, ArbitrageStrategy
- RiskManager with validation and position tracking
- Delegate-based trade execution

### Phase 3: Social Trading - COMPLETE
- TraderProfile with performance tiers (Bronze to Diamond)
- CopyTradingVault with share-based deposits
- FollowRelation for follower tracking
- Profile creation/update instructions
- Copy vault deposit/withdraw with share calculation

### Phase 4: Enterprise PaaS - COMPLETE
- EnterpriseTenant account with limits and fees
- API key management with rotation support
- Category-based market restrictions
- Revenue sharing configuration
- Volume tracking and limits

### Phase 5: DeFi Integration - COMPLETE
- YieldVault for yield-bearing collateral (Marinade, Lido, JitoSOL)
- MarginAccount for leveraged trading
- LendingPool with utilization-based rates
- Health factor tracking and liquidation logic
- Harvest yield functionality

**43/43 tests passing**

Files implemented:
- `state/orderbook.rs` - BookSide with Red-Black tree
- `state/event_heap.rs` - FillEvent, OutEvent, EventHeap
- `state/open_orders.rs` - OpenOrdersAccount with position tracking
- `state/oracle.rs` - OracleConfig, OraclePrice, ComparisonOp
- `state/trading_agent.rs` - TradingAgent, RiskParams, PositionSizing
- `state/social.rs` - TraderProfile, CopyTradingVault, FollowRelation
- `state/enterprise.rs` - EnterpriseTenant, ApiKeyRecord
- `state/defi.rs` - YieldVault, MarginAccount, LendingPool
- `instructions/place_order_v2.rs` - Full matching engine
- `instructions/resolve_market.rs` - MarketV2, oracle resolution
- `instructions/redeem.rs` - Redemption, MintTokenSet, BurnTokenSet
- `instructions/consume_events.rs` - Crank processing
- `instructions/trading_agent.rs` - Agent CRUD, delegate trading
- `instructions/social.rs` - Profile, follow, copy vault
- `instructions/enterprise.rs` - Tenant CRUD, API key rotation
- `instructions/defi.rs` - Yield vault, margin, lending pool
- `sdk/agent/src/` - TypeScript agent SDK

---

## Phase 1: Enhanced On-Chain Program

### 1.1 CLOB Implementation

**Architecture Decision**: Hybrid of OpenBook v2 (proven CRIT-BIT tree) and Manifest (space efficiency).

#### Account Structures

```rust
// Market account (~1KB fixed + dynamic orderbook)
#[account]
pub struct PredictionMarket {
    // Identity
    pub market_id: [u8; 32],
    pub bump: u8,

    // Tokens
    pub collateral_mint: Pubkey,      // USDC
    pub yes_mint: Pubkey,             // Conditional YES token
    pub no_mint: Pubkey,              // Conditional NO token
    pub vault: Pubkey,                // Collateral vault

    // Orderbook references
    pub bids: Pubkey,                 // BookSide account
    pub asks: Pubkey,                 // BookSide account
    pub event_heap: Pubkey,           // Settlement events

    // Oracle
    pub oracle_feed: Pubkey,          // Switchboard feed
    pub resolution_threshold: i128,   // Price threshold for YES outcome

    // Configuration
    pub question: String,
    pub description: String,
    pub category: String,
    pub trading_end: i64,
    pub resolution_deadline: i64,
    pub fee_bps: u16,
    pub protocol_fee_share_bps: u16,

    // State
    pub status: MarketStatus,
    pub resolved_outcome: Option<Outcome>,
    pub total_collateral: u64,
    pub seq_num: u64,

    // Timestamps
    pub created_at: i64,
    pub resolved_at: i64,
}

// BookSide for bids or asks (~90KB)
#[account(zero_copy)]
pub struct BookSide {
    pub roots: [OrderTreeRoot; 2],    // Fixed + OraclePegged
    pub nodes: OrderTreeNodes,         // 1024 nodes max
}

// User's open orders account
#[account]
pub struct OpenOrdersAccount {
    pub owner: Pubkey,
    pub market: Pubkey,
    pub delegate: Option<Pubkey>,

    // Position tracking
    pub yes_balance: u64,
    pub no_balance: u64,
    pub collateral_locked: u64,

    // Open orders (max 24 per user)
    pub open_orders: [OpenOrder; 24],
    pub open_orders_count: u8,
}

#[derive(Clone, Copy)]
pub struct OpenOrder {
    pub id: u128,
    pub client_id: u64,
    pub price: u64,                   // In basis points (0-10000)
    pub quantity: u64,
    pub side: Side,
    pub outcome: Outcome,
    pub order_type: OrderType,
    pub time_in_force: u16,           // Seconds
    pub created_at: i64,
}

pub enum OrderType {
    Limit,
    Market,
    PostOnly,
    ImmediateOrCancel,
    FillOrKill,
}
```

#### Order Matching Algorithm

```rust
pub fn place_order(
    ctx: Context<PlaceOrder>,
    params: PlaceOrderParams,
) -> Result<PlaceOrderResult> {
    let market = &mut ctx.accounts.market;
    let book = &mut ctx.accounts.book;
    let user = &mut ctx.accounts.open_orders;

    // 1. Validate market is open for trading
    require!(market.status == MarketStatus::Active, MarketNotActive);
    require!(Clock::get()?.unix_timestamp < market.trading_end, TradingEnded);

    // 2. Lock collateral for the order
    let collateral_required = calculate_collateral(
        params.side,
        params.outcome,
        params.price,
        params.quantity,
    );
    transfer_to_vault(ctx, collateral_required)?;
    user.collateral_locked += collateral_required;

    // 3. Match against opposing orders
    let opposing_side = params.side.invert();
    let mut remaining = params.quantity;
    let mut total_filled = 0u64;
    let mut fills = Vec::new();

    for best_order in book.iter_side(opposing_side) {
        if remaining == 0 { break; }

        // Check price compatibility
        if !is_price_acceptable(params.side, params.price, best_order.price) {
            break;
        }

        // Calculate fill quantity
        let fill_qty = remaining.min(best_order.quantity);
        let fill_price = best_order.price; // Maker's price

        // Record fill event
        fills.push(FillEvent {
            maker: best_order.owner,
            taker: user.owner,
            price: fill_price,
            quantity: fill_qty,
            outcome: params.outcome,
            timestamp: Clock::get()?.unix_timestamp,
        });

        // Update quantities
        remaining -= fill_qty;
        total_filled += fill_qty;

        // Mark order for removal/update
        if fill_qty == best_order.quantity {
            book.remove_order(best_order.id);
        } else {
            book.update_quantity(best_order.id, best_order.quantity - fill_qty);
        }
    }

    // 4. Post remaining quantity if limit order
    let posted_id = if remaining > 0 && params.order_type == OrderType::Limit {
        let order_id = book.insert_order(Order {
            owner: user.key(),
            price: params.price,
            quantity: remaining,
            side: params.side,
            outcome: params.outcome,
            seq_num: market.seq_num,
            created_at: Clock::get()?.unix_timestamp,
        })?;

        user.add_open_order(order_id, params)?;
        market.seq_num += 1;

        Some(order_id)
    } else {
        None
    };

    // 5. Emit events
    for fill in fills {
        emit!(fill);
    }

    Ok(PlaceOrderResult {
        order_id: posted_id,
        filled_quantity: total_filled,
        posted_quantity: remaining,
    })
}
```

#### Price Encoding

For prediction markets, prices represent probabilities (0.00 to 1.00):

```rust
// Price in basis points: 0-10000 represents 0.00% to 100.00%
pub const PRICE_SCALE: u64 = 10000;

// YES at 75% means NO at 25%
pub fn complementary_price(price: u64) -> u64 {
    PRICE_SCALE - price
}

// Collateral required for a position
pub fn calculate_collateral(
    side: Side,
    outcome: Outcome,
    price: u64,
    quantity: u64,
) -> u64 {
    match (side, outcome) {
        // Buying YES: pay price * quantity
        (Side::Buy, Outcome::Yes) => (price * quantity) / PRICE_SCALE,
        // Buying NO: pay (1-price) * quantity
        (Side::Buy, Outcome::No) => ((PRICE_SCALE - price) * quantity) / PRICE_SCALE,
        // Selling: receive price * quantity (after matching)
        _ => 0,
    }
}
```

### 1.2 Oracle Integration (Switchboard)

```rust
use switchboard_on_demand::prelude::*;

#[derive(Accounts)]
pub struct ResolveMarket<'info> {
    #[account(mut)]
    pub market: Account<'info, PredictionMarket>,

    pub oracle_feed: Account<'info, PullFeedAccountData>,

    #[account(address = market.oracle)]
    pub oracle_authority: Signer<'info>,
}

pub fn resolve_market(ctx: Context<ResolveMarket>) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let feed = &ctx.accounts.oracle_feed;
    let clock = Clock::get()?;

    // 1. Validate resolution window
    require!(
        clock.unix_timestamp >= market.trading_end,
        TradingNotEnded
    );
    require!(
        clock.unix_timestamp <= market.resolution_deadline,
        ResolutionDeadlinePassed
    );
    require!(market.status == MarketStatus::Active, MarketNotActive);

    // 2. Validate oracle data freshness
    require!(
        feed.is_result_valid(clock.slot),
        OracleDataStale
    );
    require!(
        feed.result.num_samples >= 3,
        InsufficientOracleSamples
    );

    // 3. Get oracle price (18 decimal precision)
    let oracle_value = feed.result.value;

    // 4. Determine outcome
    let outcome = if oracle_value >= market.resolution_threshold {
        Outcome::Yes
    } else {
        Outcome::No
    };

    // 5. Update market state
    market.status = MarketStatus::Resolved;
    market.resolved_outcome = Some(outcome);
    market.resolved_at = clock.unix_timestamp;

    // 6. Cancel all open orders
    cancel_all_orders(&mut ctx.accounts.bids)?;
    cancel_all_orders(&mut ctx.accounts.asks)?;

    emit!(MarketResolved {
        market: market.key(),
        outcome,
        oracle_value,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
```

### 1.3 Settlement & Redemption

```rust
pub fn redeem_winnings(ctx: Context<RedeemWinnings>) -> Result<()> {
    let market = &ctx.accounts.market;
    let user = &mut ctx.accounts.open_orders;

    require!(market.status == MarketStatus::Resolved, MarketNotResolved);

    let winning_outcome = market.resolved_outcome.unwrap();

    // Calculate winnings based on outcome
    let winnings = match winning_outcome {
        Outcome::Yes => user.yes_balance,  // 1 YES = 1 USDC
        Outcome::No => user.no_balance,    // 1 NO = 1 USDC
    };

    // Transfer from vault to user
    let seeds = market_seeds!(market);
    transfer_from_vault(
        &ctx.accounts.vault,
        &ctx.accounts.user_collateral,
        &ctx.accounts.market_authority,
        seeds,
        winnings,
    )?;

    // Zero out user's position
    user.yes_balance = 0;
    user.no_balance = 0;
    user.collateral_locked = 0;

    emit!(WinningsRedeemed {
        market: market.key(),
        user: user.owner,
        amount: winnings,
    });

    Ok(())
}
```

### 1.4 Implementation Status

| Task | Status | Notes |
|------|--------|-------|
| CLOB data structures | COMPLETE | Red-Black tree, 512 orders/side |
| Order matching engine | COMPLETE | 5 order types, price-time priority |
| Switchboard integration | COMPLETE | Manual parsing (zeroize conflict) |
| Resolution logic | COMPLETE | Oracle + manual resolution |
| Settlement & redemption | COMPLETE | Mint/burn token sets |
| Event processing (crank) | COMPLETE | Consume events, prune orders |
| AI Trading Agents | COMPLETE | On-chain + SDK |
| Social Trading | COMPLETE | Profiles, copy vaults, follows |
| Enterprise PaaS | COMPLETE | Tenants, API keys, limits |
| DeFi Integration | COMPLETE | Yield, margin, lending |
| Unit tests | COMPLETE | 43/43 passing |
| Integration tests | PENDING | Requires localnet deployment |
| Security audit | PENDING | - |

---

## Phase 2: AI Trading Agents

### 2.1 Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Agent Manager                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │ Agent Pool  │  │  Strategy   │  │   Risk      │     │
│  │  Registry   │  │   Engine    │  │  Manager    │     │
│  └─────────────┘  └─────────────┘  └─────────────┘     │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│                    Data Pipeline                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │  Market     │  │   News      │  │  Social     │     │
│  │   Data      │  │   Feed      │  │   Signals   │     │
│  └─────────────┘  └─────────────┘  └─────────────┘     │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│              On-Chain Execution Layer                    │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │  Agent      │  │   Order     │  │  Position   │     │
│  │  Accounts   │  │   Router    │  │   Tracker   │     │
│  └─────────────┘  └─────────────┘  └─────────────┘     │
└─────────────────────────────────────────────────────────┘
```

### 2.2 On-Chain Agent Account

```rust
#[account]
pub struct TradingAgent {
    pub owner: Pubkey,                // Agent creator
    pub delegate: Pubkey,             // Off-chain executor
    pub name: String,

    // Constraints
    pub max_position_size: u64,       // Per market
    pub max_total_exposure: u64,      // Across all markets
    pub allowed_markets: Vec<Pubkey>, // Whitelist (empty = all)
    pub risk_parameters: RiskParams,

    // State
    pub total_deposited: u64,
    pub total_pnl: i64,
    pub active_positions: u16,
    pub created_at: i64,
    pub last_trade_at: i64,

    // Performance tracking
    pub trades_count: u64,
    pub win_count: u64,
    pub volume_traded: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct RiskParams {
    pub max_drawdown_bps: u16,        // Stop trading if exceeded
    pub max_daily_loss: u64,
    pub min_probability_edge: u16,    // Minimum edge to trade (bps)
    pub position_sizing: PositionSizing,
}

pub enum PositionSizing {
    Fixed(u64),                       // Fixed size per trade
    Kelly { fraction: u16 },          // Kelly criterion fraction
    Proportional { risk_bps: u16 },   // % of bankroll per trade
}
```

### 2.3 Off-Chain Agent Service

```typescript
// Agent configuration
interface AgentConfig {
  id: string;
  strategy: StrategyType;
  riskParams: RiskParams;
  dataSources: DataSource[];
  executionParams: ExecutionParams;
}

// Strategy interface
interface Strategy {
  analyze(market: Market, data: MarketData): Signal | null;
  calculatePosition(signal: Signal, portfolio: Portfolio): Order | null;
}

// Example: News-based strategy
class NewsStrategy implements Strategy {
  private nlpModel: NLPModel;

  analyze(market: Market, data: MarketData): Signal | null {
    const news = data.news.filter(n => n.relevance > 0.8);
    if (news.length === 0) return null;

    const sentiment = this.nlpModel.analyzeSentiment(news);
    const currentPrice = market.yesPrice;

    // Generate signal if sentiment diverges from price
    const impliedProbability = sentiment.probability;
    const edge = impliedProbability - currentPrice;

    if (Math.abs(edge) > this.minEdge) {
      return {
        market: market.id,
        direction: edge > 0 ? 'buy_yes' : 'buy_no',
        confidence: Math.abs(edge),
        reason: sentiment.summary,
      };
    }

    return null;
  }
}

// Execution service
class AgentExecutor {
  async executeSignal(agent: TradingAgent, signal: Signal) {
    // 1. Validate against risk parameters
    if (!this.riskManager.validateTrade(agent, signal)) {
      return { executed: false, reason: 'risk_limit' };
    }

    // 2. Calculate optimal order
    const order = this.positionSizer.calculate(agent, signal);

    // 3. Submit to chain
    const tx = await this.program.methods
      .placeOrder(order)
      .accounts({
        market: signal.market,
        openOrders: agent.openOrdersAccount,
        delegate: this.wallet.publicKey,
      })
      .rpc();

    // 4. Record trade
    await this.recordTrade(agent, signal, order, tx);

    return { executed: true, txId: tx };
  }
}
```

### 2.4 AI Market Creation

```rust
#[account]
pub struct AIMarketProposal {
    pub proposer: Pubkey,             // AI agent or human
    pub question: String,
    pub description: String,
    pub category: String,
    pub suggested_oracle: Pubkey,
    pub resolution_criteria: String,

    // AI metadata
    pub confidence_score: u16,        // 0-10000
    pub data_sources: Vec<String>,
    pub reasoning: String,

    // Governance
    pub votes_for: u64,
    pub votes_against: u64,
    pub status: ProposalStatus,
    pub created_at: i64,
    pub voting_end: i64,
}

// Off-chain: Market opportunity detection
async function detectMarketOpportunity(
  newsEvent: NewsEvent
): Promise<MarketProposal | null> {
  // 1. Analyze news for predictable outcomes
  const analysis = await analyzeNewsForPrediction(newsEvent);
  if (analysis.confidence < 0.7) return null;

  // 2. Check for existing similar markets
  const existingMarkets = await findSimilarMarkets(analysis.topic);
  if (existingMarkets.length > 0) return null;

  // 3. Find appropriate oracle
  const oracle = await findOracle(analysis.dataType);
  if (!oracle) return null;

  // 4. Generate market proposal
  return {
    question: analysis.question,
    description: analysis.context,
    category: analysis.category,
    oracle: oracle.publicKey,
    resolutionCriteria: analysis.criteria,
    confidence: analysis.confidence,
    dataSources: analysis.sources,
    reasoning: analysis.reasoning,
  };
}
```

---

## Phase 3: Social Trading with ZK Privacy

### 3.1 ZK Leaderboard Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   ZK Proof System                        │
│                                                          │
│  Trader submits:                                         │
│  - Encrypted performance data                           │
│  - ZK proof of ranking validity                         │
│                                                          │
│  Verifier checks:                                        │
│  - Proof is valid                                       │
│  - Ranking is consistent                                │
│  - No double-counting                                   │
└─────────────────────────────────────────────────────────┘
```

### 3.2 Private Performance Tracking

```rust
// On-chain: Commitment to performance
#[account]
pub struct PrivateTraderProfile {
    pub owner: Pubkey,
    pub commitment: [u8; 32],         // Pedersen commitment to PnL
    pub proof_timestamp: i64,
    pub tier: PerformanceTier,        // Derived from ZK proof
    pub follower_count: u32,
}

// Off-chain: Generate ZK proof of performance tier
struct PerformanceProof {
    // Public inputs
    tier: PerformanceTier,
    period: TimePeriod,

    // Private inputs (not revealed)
    actual_pnl: i128,
    trade_history: Vec<Trade>,

    // Proof
    proof: Groth16Proof,
}

// Circuit: Prove PnL is in tier range without revealing exact value
fn performance_tier_circuit(
    public_tier: PerformanceTier,
    private_pnl: i128,
    private_trades: Vec<Trade>,
) -> bool {
    // 1. Verify trade history sums to PnL
    let computed_pnl = private_trades.iter()
        .map(|t| t.realized_pnl)
        .sum();
    assert_eq!(computed_pnl, private_pnl);

    // 2. Verify PnL falls in tier range
    let (min, max) = tier_range(public_tier);
    assert!(private_pnl >= min && private_pnl < max);

    true
}
```

### 3.3 Anonymous Copy-Trading

```rust
#[account]
pub struct CopyTradingVault {
    pub leader_commitment: [u8; 32],  // ZK commitment to leader identity
    pub total_deposited: u64,
    pub follower_count: u32,
    pub fee_bps: u16,

    // Performance tracking (public)
    pub total_pnl: i64,
    pub sharpe_ratio: i32,            // Scaled by 1000
    pub max_drawdown_bps: u16,
}

// Follower deposits without knowing leader's identity
pub fn follow_trader(
    ctx: Context<FollowTrader>,
    amount: u64,
    leader_commitment: [u8; 32],
) -> Result<()> {
    // Verify vault matches commitment
    require!(
        ctx.accounts.vault.leader_commitment == leader_commitment,
        InvalidLeaderCommitment
    );

    // Deposit to vault
    transfer_to_vault(ctx, amount)?;

    // Mint share tokens to follower
    mint_shares(ctx, amount)?;

    Ok(())
}

// Leader executes trades on behalf of vault
pub fn execute_copy_trade(
    ctx: Context<ExecuteCopyTrade>,
    order: OrderParams,
    leader_proof: LeaderProof,
) -> Result<()> {
    // Verify ZK proof that signer is the leader
    require!(
        verify_leader_proof(&ctx.accounts.vault, &leader_proof),
        InvalidLeaderProof
    );

    // Execute trade for vault
    place_order_for_vault(ctx, order)?;

    Ok(())
}
```

---

## Phase 4: Prediction-as-a-Service (PaaS)

### 4.1 White-Label API

```typescript
// REST API for enterprise clients
interface PaaSAPI {
  // Market Management
  createMarket(params: CreateMarketParams): Promise<Market>;
  listMarkets(filters: MarketFilters): Promise<Market[]>;
  getMarket(id: string): Promise<Market>;
  resolveMarket(id: string, outcome: Outcome): Promise<void>;

  // Trading
  placeOrder(marketId: string, order: OrderParams): Promise<Order>;
  cancelOrder(orderId: string): Promise<void>;
  getOrderBook(marketId: string): Promise<OrderBook>;

  // User Management
  createUser(params: CreateUserParams): Promise<User>;
  getBalance(userId: string): Promise<Balance>;
  deposit(userId: string, amount: number): Promise<void>;
  withdraw(userId: string, amount: number): Promise<void>;

  // Analytics
  getMarketAnalytics(marketId: string): Promise<Analytics>;
  getUserAnalytics(userId: string): Promise<UserAnalytics>;
}

// SDK for integration
class PolyguardSDK {
  constructor(
    private apiKey: string,
    private endpoint: string = 'https://api.polyguard.cc'
  ) {}

  async createMarket(params: CreateMarketParams): Promise<Market> {
    const response = await fetch(`${this.endpoint}/v1/markets`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.apiKey}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(params),
    });
    return response.json();
  }

  // WebSocket for real-time updates
  subscribeToMarket(marketId: string, callback: (update: MarketUpdate) => void) {
    const ws = new WebSocket(`${this.wsEndpoint}/markets/${marketId}`);
    ws.onmessage = (event) => callback(JSON.parse(event.data));
    return ws;
  }
}
```

### 4.2 Enterprise Features

```rust
// On-chain: Enterprise tenant account
#[account]
pub struct EnterpriseTenant {
    pub owner: Pubkey,
    pub name: String,
    pub api_key_hash: [u8; 32],

    // Limits
    pub max_markets: u32,
    pub max_daily_volume: u64,
    pub allowed_categories: Vec<String>,

    // Fees
    pub fee_override_bps: Option<u16>,
    pub revenue_share_bps: u16,

    // Stats
    pub markets_created: u32,
    pub total_volume: u64,
    pub fees_collected: u64,
}

// Custom market creation for tenants
pub fn create_tenant_market(
    ctx: Context<CreateTenantMarket>,
    params: CreateMarketParams,
) -> Result<()> {
    let tenant = &ctx.accounts.tenant;

    // Validate tenant limits
    require!(
        tenant.markets_created < tenant.max_markets,
        TenantMarketLimitExceeded
    );

    // Apply custom fees if set
    let fee_bps = tenant.fee_override_bps.unwrap_or(DEFAULT_FEE_BPS);

    // Create market with tenant branding
    create_market_internal(ctx, params, fee_bps, tenant.key())?;

    Ok(())
}
```

---

## Phase 5: DeFi Integration

### 5.1 Yield-Bearing Collateral

```rust
// Support for yield-bearing tokens as collateral
#[account]
pub struct YieldVault {
    pub market: Pubkey,
    pub underlying_vault: Pubkey,     // e.g., Marinade stSOL vault
    pub yield_source: YieldSource,

    pub total_deposited: u64,
    pub yield_accrued: u64,
    pub last_harvest: i64,
}

pub enum YieldSource {
    Marinade,                         // stSOL
    Lido,                             // stSOL
    JitoSOL,                          // jitoSOL
    UXDProtocol,                      // UXD stablecoin
}

// Deposit yield-bearing token
pub fn deposit_yield_collateral(
    ctx: Context<DepositYieldCollateral>,
    amount: u64,
) -> Result<()> {
    // 1. Transfer yield token to vault
    transfer_to_yield_vault(ctx, amount)?;

    // 2. Calculate base value (accounting for yield token price)
    let base_value = calculate_base_value(
        ctx.accounts.yield_vault.yield_source,
        amount,
    )?;

    // 3. Credit user's collateral balance
    ctx.accounts.open_orders.collateral_balance += base_value;

    // 4. Track yield accrual
    ctx.accounts.yield_vault.total_deposited += amount;

    Ok(())
}

// Harvest and distribute yield
pub fn harvest_yield(ctx: Context<HarvestYield>) -> Result<()> {
    let vault = &mut ctx.accounts.yield_vault;

    // 1. Calculate yield since last harvest
    let current_value = get_yield_token_value(vault.yield_source)?;
    let yield_earned = current_value - vault.total_deposited;

    // 2. Distribute to market participants pro-rata
    distribute_yield(ctx, yield_earned)?;

    // 3. Update state
    vault.yield_accrued += yield_earned;
    vault.last_harvest = Clock::get()?.unix_timestamp;

    Ok(())
}
```

### 5.2 Leveraged Predictions

```rust
// Margin account for leveraged trading
#[account]
pub struct MarginAccount {
    pub owner: Pubkey,
    pub collateral: u64,              // Deposited collateral
    pub borrowed: u64,                // Borrowed amount
    pub max_leverage: u8,             // e.g., 3x

    // Health tracking
    pub health_factor: u16,           // Scaled by 10000, < 10000 = liquidatable
    pub last_update: i64,
}

// Place leveraged order
pub fn place_leveraged_order(
    ctx: Context<PlaceLeveragedOrder>,
    params: LeveragedOrderParams,
) -> Result<()> {
    let margin = &mut ctx.accounts.margin_account;

    // 1. Calculate required margin
    let position_size = params.quantity * params.price / PRICE_SCALE;
    let required_margin = position_size / (params.leverage as u64);

    require!(
        margin.collateral >= required_margin,
        InsufficientMargin
    );
    require!(
        params.leverage <= margin.max_leverage,
        LeverageExceedsMax
    );

    // 2. Borrow remaining from lending pool
    let borrow_amount = position_size - required_margin;
    borrow_from_pool(ctx, borrow_amount)?;
    margin.borrowed += borrow_amount;

    // 3. Place order with full position size
    place_order(ctx, OrderParams {
        quantity: params.quantity,
        price: params.price,
        ..params.order_params
    })?;

    // 4. Update health factor
    margin.health_factor = calculate_health(margin)?;

    Ok(())
}

// Liquidation
pub fn liquidate_position(ctx: Context<Liquidate>) -> Result<()> {
    let margin = &ctx.accounts.margin_account;

    require!(
        margin.health_factor < LIQUIDATION_THRESHOLD,
        PositionHealthy
    );

    // 1. Close all positions
    close_all_positions(ctx)?;

    // 2. Repay borrowed amount
    repay_to_pool(ctx, margin.borrowed)?;

    // 3. Send liquidation bonus to liquidator
    let bonus = margin.collateral * LIQUIDATION_BONUS_BPS / 10000;
    transfer_to_liquidator(ctx, bonus)?;

    // 4. Return remaining to user
    let remaining = margin.collateral - margin.borrowed - bonus;
    if remaining > 0 {
        transfer_to_user(ctx, remaining)?;
    }

    Ok(())
}
```

---

## Technical Stack

### On-Chain
- **Framework**: Anchor 0.30+
- **Runtime**: Solana 1.18+
- **Token Standard**: SPL Token / Token-2022
- **Oracle**: Switchboard v2 On-Demand

### Off-Chain
- **Backend**: Rust (Axum) or Node.js (Fastify)
- **Database**: PostgreSQL + TimescaleDB
- **Cache**: Redis
- **Queue**: RabbitMQ or Kafka
- **AI/ML**: Python (FastAPI) + PyTorch

### Frontend
- **Framework**: Next.js 14+
- **State**: TanStack Query + Zustand
- **Wallet**: Solana Wallet Adapter
- **Charts**: Lightweight Charts + D3

---

## Security Considerations

1. **Smart Contract Audits**: Engage OtterSec, Neodyme, or similar for each phase
2. **Oracle Manipulation**: Require multiple oracle confirmations, use TWAP
3. **Front-Running**: Implement commit-reveal for large orders
4. **Reentrancy**: Use Anchor's built-in checks + additional guards
5. **Flash Loans**: Resolution delay prevents flash loan attacks
6. **Rate Limiting**: On-chain CU limits + off-chain API rate limits

---

## Cost Estimates

| Component | One-Time | Monthly |
|-----------|----------|---------|
| Development (Phase 1) | $80-120k | - |
| Development (Phase 2-5) | $150-250k | - |
| Security Audits | $50-100k | - |
| Infrastructure | - | $2-5k |
| Oracle Feeds | - | $100-500 |
| **Total** | **$280-470k** | **$2-5.5k** |

---

## Success Metrics

| Phase | KPI | Target |
|-------|-----|--------|
| 1 | Markets created | 50+ |
| 1 | Daily volume | $100k+ |
| 2 | Active AI agents | 20+ |
| 2 | Agent-driven volume | 30%+ |
| 3 | Followers using ZK | 500+ |
| 4 | Enterprise clients | 5+ |
| 5 | TVL in yield vaults | $1M+ |

---

## References

- [OpenBook v2 Source](https://github.com/openbook-dex/openbook-v2)
- [Manifest Source](https://github.com/CKS-Systems/manifest)
- [Switchboard v2 Docs](https://docs.switchboard.xyz/)
- [Hedgehog Markets](https://github.com/Hedgehog-Markets/hedgehog-escrow)
- [Solana Program Library](https://github.com/solana-labs/solana-program-library)
