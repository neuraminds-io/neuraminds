import { PublicKey } from '@solana/web3.js';

/** Position sizing strategy */
export enum PositionSizing {
  Fixed = 0,
  Kelly = 1,
  Proportional = 2,
}

/** Agent status */
export enum AgentStatus {
  Active = 0,
  Paused = 1,
  Stopped = 2,
}

/** Order side */
export enum Side {
  Buy = 0,
  Sell = 1,
}

/** Market outcome */
export enum Outcome {
  Yes = 0,
  No = 1,
}

/** Order type */
export enum OrderType {
  Limit = 0,
  Market = 1,
  PostOnly = 2,
  ImmediateOrCancel = 3,
  FillOrKill = 4,
}

/** Risk parameters configuration */
export interface RiskParams {
  maxDrawdownBps: number;
  maxDailyLoss: bigint;
  minEdgeBps: number;
  positionSizing: PositionSizing;
  sizingParam: bigint;
}

/** Trading agent account data */
export interface TradingAgentAccount {
  owner: PublicKey;
  delegate: PublicKey;
  name: string;
  bump: number;
  status: AgentStatus;
  version: number;
  maxPositionSize: bigint;
  maxTotalExposure: bigint;
  riskParams: RiskParams;
  totalDeposited: bigint;
  availableBalance: bigint;
  lockedBalance: bigint;
  totalPnl: bigint;
  highWaterMark: bigint;
  currentDrawdown: bigint;
  dailyLoss: bigint;
  lastDay: bigint;
  activePositions: number;
  tradesCount: bigint;
  winCount: bigint;
  volumeTraded: bigint;
  createdAt: bigint;
  lastTradeAt: bigint;
  allowedMarketsCount: number;
  allowedMarkets: PublicKey[];
}

/** Agent creation parameters */
export interface CreateAgentParams {
  name: string;
  delegate: PublicKey;
  maxPositionSize: bigint;
  maxTotalExposure: bigint;
  riskParams: RiskParams;
}

/** Order parameters */
export interface OrderParams {
  side: Side;
  outcome: Outcome;
  price: number; // basis points (0-10000)
  quantity: bigint;
  orderType: OrderType;
  clientOrderId?: bigint;
}

/** Trading signal from strategy */
export interface Signal {
  market: PublicKey;
  direction: 'buy_yes' | 'buy_no' | 'sell_yes' | 'sell_no';
  confidence: number; // 0-1
  targetPrice: number; // basis points
  reason?: string;
}

/** Market data for strategy analysis */
export interface MarketData {
  market: PublicKey;
  yesPrice: number;
  noPrice: number;
  volume24h: bigint;
  liquidity: bigint;
  lastUpdate: number;
  metadata?: Record<string, unknown>;
}

/** Trade result */
export interface TradeResult {
  success: boolean;
  txId?: string;
  filledQuantity?: bigint;
  avgPrice?: number;
  error?: string;
}

/** Agent performance metrics */
export interface AgentMetrics {
  totalPnl: bigint;
  winRate: number; // 0-1
  tradesCount: bigint;
  avgPnlPerTrade: bigint;
  sharpeRatio?: number;
  maxDrawdown: number; // basis points
  volumeTraded: bigint;
}
