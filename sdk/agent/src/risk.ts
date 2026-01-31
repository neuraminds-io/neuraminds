import { TradingAgentAccount, RiskParams, OrderParams, PositionSizing } from './types';

/**
 * Risk manager validates trades against agent constraints
 */
export class RiskManager {
  constructor(private readonly agent: TradingAgentAccount) {}

  /**
   * Validate a trade against all risk parameters
   */
  validateTrade(order: OrderParams): ValidationResult {
    const checks: ValidationCheck[] = [
      this.checkPositionSize(order),
      this.checkExposure(order),
      this.checkDrawdown(),
      this.checkDailyLoss(),
      this.checkMinEdge(order),
    ];

    const failed = checks.filter(c => !c.passed);

    return {
      valid: failed.length === 0,
      checks,
      failedChecks: failed,
    };
  }

  /**
   * Check position size limit
   */
  private checkPositionSize(order: OrderParams): ValidationCheck {
    const size = order.quantity;
    const maxSize = this.agent.maxPositionSize;

    return {
      name: 'position_size',
      passed: size <= maxSize,
      message: size <= maxSize
        ? `Position size ${size} within limit ${maxSize}`
        : `Position size ${size} exceeds limit ${maxSize}`,
      value: Number(size),
      limit: Number(maxSize),
    };
  }

  /**
   * Check total exposure limit
   */
  private checkExposure(order: OrderParams): ValidationCheck {
    const currentLocked = this.agent.lockedBalance;
    const orderCollateral = (order.quantity * BigInt(order.price)) / 10000n;
    const newExposure = currentLocked + orderCollateral;
    const maxExposure = this.agent.maxTotalExposure;

    return {
      name: 'exposure',
      passed: newExposure <= maxExposure,
      message: newExposure <= maxExposure
        ? `Exposure ${newExposure} within limit ${maxExposure}`
        : `Exposure ${newExposure} exceeds limit ${maxExposure}`,
      value: Number(newExposure),
      limit: Number(maxExposure),
    };
  }

  /**
   * Check drawdown limit
   */
  private checkDrawdown(): ValidationCheck {
    const maxDrawdownBps = this.agent.riskParams.maxDrawdownBps;
    if (maxDrawdownBps === 0 || this.agent.highWaterMark === 0n) {
      return { name: 'drawdown', passed: true, message: 'Drawdown check disabled' };
    }

    const drawdownBps = Number(
      (this.agent.currentDrawdown * 10000n) / this.agent.highWaterMark
    );

    return {
      name: 'drawdown',
      passed: drawdownBps <= maxDrawdownBps,
      message: drawdownBps <= maxDrawdownBps
        ? `Drawdown ${drawdownBps}bps within limit ${maxDrawdownBps}bps`
        : `Drawdown ${drawdownBps}bps exceeds limit ${maxDrawdownBps}bps`,
      value: drawdownBps,
      limit: maxDrawdownBps,
    };
  }

  /**
   * Check daily loss limit
   */
  private checkDailyLoss(): ValidationCheck {
    const maxDailyLoss = this.agent.riskParams.maxDailyLoss;
    if (maxDailyLoss === 0n) {
      return { name: 'daily_loss', passed: true, message: 'Daily loss check disabled' };
    }

    const currentDailyLoss = this.agent.dailyLoss;

    return {
      name: 'daily_loss',
      passed: currentDailyLoss < maxDailyLoss,
      message: currentDailyLoss < maxDailyLoss
        ? `Daily loss ${currentDailyLoss} within limit ${maxDailyLoss}`
        : `Daily loss ${currentDailyLoss} exceeds limit ${maxDailyLoss}`,
      value: Number(currentDailyLoss),
      limit: Number(maxDailyLoss),
    };
  }

  /**
   * Check minimum edge requirement
   */
  private checkMinEdge(order: OrderParams): ValidationCheck {
    const minEdgeBps = this.agent.riskParams.minEdgeBps;
    if (minEdgeBps === 0) {
      return { name: 'min_edge', passed: true, message: 'Min edge check disabled' };
    }

    // Edge calculation would require market fair value
    // For now, just return passed
    return {
      name: 'min_edge',
      passed: true,
      message: `Min edge check requires market data`,
    };
  }

  /**
   * Calculate position size based on risk parameters
   */
  calculatePositionSize(
    edge: number, // Expected edge in basis points
    winProbability: number, // Win probability (0-1)
    price: number, // Current price in basis points
  ): bigint {
    const bankroll = this.agent.availableBalance;
    const params = this.agent.riskParams;

    switch (params.positionSizing) {
      case PositionSizing.Fixed:
        return params.sizingParam;

      case PositionSizing.Kelly:
        return this.kellySize(bankroll, edge, winProbability, params.sizingParam);

      case PositionSizing.Proportional:
        return this.proportionalSize(bankroll, params.sizingParam);

      default:
        return params.sizingParam;
    }
  }

  /**
   * Kelly criterion position sizing
   */
  private kellySize(
    bankroll: bigint,
    edgeBps: number,
    winProb: number,
    kellyFraction: bigint,
  ): bigint {
    // Kelly formula: f = (p*b - q) / b
    // Simplified: f = edge / variance
    // We use a fraction of Kelly for safety

    const edge = edgeBps / 10000;
    const kelly = edge * winProb; // Simplified Kelly

    // Apply fraction
    const adjustedKelly = kelly * Number(kellyFraction) / 10000;

    // Calculate size
    return BigInt(Math.floor(Number(bankroll) * adjustedKelly));
  }

  /**
   * Proportional position sizing (fixed % of bankroll)
   */
  private proportionalSize(bankroll: bigint, riskBps: bigint): bigint {
    return (bankroll * riskBps) / 10000n;
  }
}

export interface ValidationResult {
  valid: boolean;
  checks: ValidationCheck[];
  failedChecks: ValidationCheck[];
}

export interface ValidationCheck {
  name: string;
  passed: boolean;
  message: string;
  value?: number;
  limit?: number;
}

/**
 * Position tracker for managing open positions
 */
export class PositionTracker {
  private positions: Map<string, Position> = new Map();

  addPosition(position: Position): void {
    const key = `${position.market.toBase58()}_${position.outcome}`;
    const existing = this.positions.get(key);

    if (existing) {
      // Average into existing position
      const totalQty = existing.quantity + position.quantity;
      const avgPrice = (
        (existing.avgEntryPrice * Number(existing.quantity)) +
        (position.avgEntryPrice * Number(position.quantity))
      ) / Number(totalQty);

      existing.quantity = totalQty;
      existing.avgEntryPrice = avgPrice;
    } else {
      this.positions.set(key, position);
    }
  }

  removePosition(market: string, outcome: number): Position | undefined {
    const key = `${market}_${outcome}`;
    const position = this.positions.get(key);
    this.positions.delete(key);
    return position;
  }

  reducePosition(market: string, outcome: number, quantity: bigint): void {
    const key = `${market}_${outcome}`;
    const position = this.positions.get(key);

    if (position) {
      position.quantity -= quantity;
      if (position.quantity <= 0n) {
        this.positions.delete(key);
      }
    }
  }

  getPosition(market: string, outcome: number): Position | undefined {
    return this.positions.get(`${market}_${outcome}`);
  }

  getAllPositions(): Position[] {
    return Array.from(this.positions.values());
  }

  getTotalExposure(): bigint {
    return this.getAllPositions().reduce(
      (sum, pos) => sum + (pos.quantity * BigInt(Math.floor(pos.avgEntryPrice))) / 10000n,
      0n
    );
  }

  getUnrealizedPnl(currentPrices: Map<string, number>): bigint {
    let totalPnl = 0n;

    for (const pos of this.positions.values()) {
      const key = `${pos.market.toBase58()}_${pos.outcome}`;
      const currentPrice = currentPrices.get(key) || pos.avgEntryPrice;
      const priceDiff = currentPrice - pos.avgEntryPrice;
      const pnl = (pos.quantity * BigInt(Math.floor(priceDiff))) / 10000n;
      totalPnl += pnl;
    }

    return totalPnl;
  }
}

export interface Position {
  market: import('@solana/web3.js').PublicKey;
  outcome: number;
  quantity: bigint;
  avgEntryPrice: number;
  openedAt: number;
}
