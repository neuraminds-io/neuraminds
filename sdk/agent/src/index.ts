// Polyguard AI Trading Agent SDK

// Core agent
export { TradingAgent, createAgent } from './agent';

// Types
export {
  PositionSizing,
  AgentStatus,
  Side,
  Outcome,
  OrderType,
  RiskParams,
  TradingAgentAccount,
  CreateAgentParams,
  OrderParams,
  Signal,
  MarketData,
  TradeResult,
  AgentMetrics,
} from './types';

// Strategies
export {
  Strategy,
  MomentumStrategy,
  MeanReversionStrategy,
  ArbitrageStrategy,
  CompositeStrategy,
} from './strategy';

// Risk management
export {
  RiskManager,
  PositionTracker,
  ValidationResult,
  ValidationCheck,
  Position,
} from './risk';
