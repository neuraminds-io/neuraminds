import { PublicKey } from '@solana/web3.js';
import { Signal, MarketData, OrderParams, Side, Outcome, OrderType } from './types';

/**
 * Base strategy interface that all trading strategies must implement
 */
export interface Strategy {
  /** Strategy name */
  name: string;

  /** Analyze market and generate signal */
  analyze(market: PublicKey, data: MarketData): Signal | null;

  /** Convert signal to order parameters */
  toOrder(signal: Signal, bankroll: bigint): OrderParams | null;
}

/**
 * Momentum strategy - trades based on price movement
 */
export class MomentumStrategy implements Strategy {
  name = 'momentum';

  constructor(
    private readonly minEdge: number = 0.05, // 5% minimum edge
    private readonly lookbackPeriod: number = 24, // hours
  ) {}

  analyze(market: PublicKey, data: MarketData): Signal | null {
    // Simple momentum: if price moved significantly, expect continuation
    // In production, would analyze historical price data
    const currentPrice = data.yesPrice;

    // Placeholder momentum calculation
    const momentum = this.calculateMomentum(data);

    if (Math.abs(momentum) < this.minEdge) {
      return null;
    }

    return {
      market,
      direction: momentum > 0 ? 'buy_yes' : 'buy_no',
      confidence: Math.min(Math.abs(momentum) / this.minEdge, 1),
      targetPrice: currentPrice + (momentum > 0 ? 100 : -100),
      reason: `Momentum signal: ${momentum > 0 ? 'bullish' : 'bearish'}`,
    };
  }

  toOrder(signal: Signal, bankroll: bigint): OrderParams | null {
    if (signal.confidence < 0.5) {
      return null;
    }

    const riskAmount = bankroll * BigInt(Math.floor(signal.confidence * 100)) / 10000n;
    const quantity = riskAmount * 10000n / BigInt(signal.targetPrice);

    return {
      side: signal.direction.startsWith('buy') ? Side.Buy : Side.Sell,
      outcome: signal.direction.includes('yes') ? Outcome.Yes : Outcome.No,
      price: signal.targetPrice,
      quantity,
      orderType: OrderType.Limit,
    };
  }

  private calculateMomentum(data: MarketData): number {
    // Simplified momentum calculation
    // In production, would use historical price data
    const midPrice = (data.yesPrice + (10000 - data.noPrice)) / 2;
    return (midPrice - 5000) / 5000; // Deviation from 50%
  }
}

/**
 * Mean reversion strategy - trades expecting price to return to mean
 */
export class MeanReversionStrategy implements Strategy {
  name = 'mean_reversion';

  constructor(
    private readonly deviationThreshold: number = 0.15, // 15% from mean
    private readonly meanPrice: number = 5000, // 50% default mean
  ) {}

  analyze(market: PublicKey, data: MarketData): Signal | null {
    const currentPrice = data.yesPrice;
    const deviation = (currentPrice - this.meanPrice) / this.meanPrice;

    if (Math.abs(deviation) < this.deviationThreshold) {
      return null;
    }

    // Trade against the deviation
    return {
      market,
      direction: deviation > 0 ? 'buy_no' : 'buy_yes',
      confidence: Math.min(Math.abs(deviation) / this.deviationThreshold, 1) * 0.8,
      targetPrice: this.meanPrice,
      reason: `Mean reversion: price ${deviation > 0 ? 'above' : 'below'} mean`,
    };
  }

  toOrder(signal: Signal, bankroll: bigint): OrderParams | null {
    if (signal.confidence < 0.4) {
      return null;
    }

    const riskAmount = bankroll * BigInt(Math.floor(signal.confidence * 50)) / 10000n;
    const quantity = riskAmount * 10000n / BigInt(signal.targetPrice);

    return {
      side: signal.direction.startsWith('buy') ? Side.Buy : Side.Sell,
      outcome: signal.direction.includes('yes') ? Outcome.Yes : Outcome.No,
      price: signal.targetPrice,
      quantity,
      orderType: OrderType.Limit,
    };
  }
}

/**
 * Arbitrage strategy - exploits price differences
 */
export class ArbitrageStrategy implements Strategy {
  name = 'arbitrage';

  constructor(
    private readonly minSpread: number = 50, // 0.5% minimum spread
  ) {}

  analyze(market: PublicKey, data: MarketData): Signal | null {
    // In prediction markets, YES + NO should equal 1
    // Any deviation is arbitrage opportunity
    const impliedSum = data.yesPrice + data.noPrice;
    const deviation = impliedSum - 10000;

    if (Math.abs(deviation) < this.minSpread) {
      return null;
    }

    // If sum > 10000, both are overpriced - sell both
    // If sum < 10000, both are underpriced - buy both
    // For simplicity, we target the more mispriced side
    if (deviation > 0) {
      // Overpriced - sell the higher one
      return {
        market,
        direction: data.yesPrice > data.noPrice ? 'sell_yes' : 'sell_no',
        confidence: Math.min(Math.abs(deviation) / 100, 1),
        targetPrice: data.yesPrice > data.noPrice ? data.yesPrice - 50 : data.noPrice - 50,
        reason: `Arbitrage: market overpriced by ${deviation}bps`,
      };
    } else {
      // Underpriced - buy the lower one
      return {
        market,
        direction: data.yesPrice < data.noPrice ? 'buy_yes' : 'buy_no',
        confidence: Math.min(Math.abs(deviation) / 100, 1),
        targetPrice: data.yesPrice < data.noPrice ? data.yesPrice + 50 : data.noPrice + 50,
        reason: `Arbitrage: market underpriced by ${-deviation}bps`,
      };
    }
  }

  toOrder(signal: Signal, bankroll: bigint): OrderParams | null {
    // Arbitrage uses larger positions due to lower risk
    const riskAmount = bankroll * BigInt(Math.floor(signal.confidence * 200)) / 10000n;
    const quantity = riskAmount * 10000n / BigInt(signal.targetPrice);

    return {
      side: signal.direction.startsWith('buy') ? Side.Buy : Side.Sell,
      outcome: signal.direction.includes('yes') ? Outcome.Yes : Outcome.No,
      price: signal.targetPrice,
      quantity,
      orderType: OrderType.ImmediateOrCancel, // Execute immediately
    };
  }
}

/**
 * Composite strategy that combines multiple strategies
 */
export class CompositeStrategy implements Strategy {
  name = 'composite';

  constructor(
    private readonly strategies: Array<{ strategy: Strategy; weight: number }>,
  ) {}

  analyze(market: PublicKey, data: MarketData): Signal | null {
    const signals: Array<{ signal: Signal; weight: number }> = [];

    for (const { strategy, weight } of this.strategies) {
      const signal = strategy.analyze(market, data);
      if (signal) {
        signals.push({ signal, weight });
      }
    }

    if (signals.length === 0) {
      return null;
    }

    // Find consensus direction
    let yesScore = 0;
    let noScore = 0;

    for (const { signal, weight } of signals) {
      const score = signal.confidence * weight;
      if (signal.direction.includes('yes')) {
        yesScore += score;
      } else {
        noScore += score;
      }
    }

    const totalWeight = this.strategies.reduce((sum, s) => sum + s.weight, 0);
    const normalizedYes = yesScore / totalWeight;
    const normalizedNo = noScore / totalWeight;

    if (Math.max(normalizedYes, normalizedNo) < 0.3) {
      return null;
    }

    const direction = normalizedYes > normalizedNo ? 'buy_yes' : 'buy_no';
    const avgPrice = signals.reduce((sum, s) => sum + s.signal.targetPrice * s.weight, 0) /
                     signals.reduce((sum, s) => sum + s.weight, 0);

    return {
      market,
      direction: direction as Signal['direction'],
      confidence: Math.max(normalizedYes, normalizedNo),
      targetPrice: Math.round(avgPrice),
      reason: `Composite: ${signals.length} strategies agree`,
    };
  }

  toOrder(signal: Signal, bankroll: bigint): OrderParams | null {
    const riskAmount = bankroll * BigInt(Math.floor(signal.confidence * 100)) / 10000n;
    const quantity = riskAmount * 10000n / BigInt(signal.targetPrice);

    return {
      side: signal.direction.startsWith('buy') ? Side.Buy : Side.Sell,
      outcome: signal.direction.includes('yes') ? Outcome.Yes : Outcome.No,
      price: signal.targetPrice,
      quantity,
      orderType: OrderType.Limit,
    };
  }
}
