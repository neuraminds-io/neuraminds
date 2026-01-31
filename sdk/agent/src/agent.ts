import {
  Connection,
  PublicKey,
  Keypair,
  Transaction,
  TransactionInstruction,
  SendTransactionError,
} from '@solana/web3.js';
import { Program, AnchorProvider, BN } from '@coral-xyz/anchor';
import {
  TradingAgentAccount,
  CreateAgentParams,
  OrderParams,
  TradeResult,
  MarketData,
  Signal,
  AgentMetrics,
  AgentStatus,
} from './types';
import { Strategy } from './strategy';
import { RiskManager, PositionTracker, Position } from './risk';

// Retry configuration
const DEFAULT_RETRY_CONFIG = {
  maxRetries: 3,
  baseDelayMs: 500,
  maxDelayMs: 10000,
};

// Transient errors that warrant retry
const TRANSIENT_ERROR_PATTERNS = [
  'blockhash not found',
  'block height exceeded',
  'timeout',
  'socket hang up',
  'ECONNREFUSED',
  'ENOTFOUND',
  'ETIMEDOUT',
  'Network request failed',
  '429', // Rate limited
  '502', // Bad gateway
  '503', // Service unavailable
  '504', // Gateway timeout
];

function isTransientError(error: unknown): boolean {
  const message = error instanceof Error ? error.message : String(error);
  return TRANSIENT_ERROR_PATTERNS.some(pattern =>
    message.toLowerCase().includes(pattern.toLowerCase())
  );
}

async function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function withRetry<T>(
  fn: () => Promise<T>,
  config = DEFAULT_RETRY_CONFIG,
): Promise<T> {
  let lastError: unknown;

  for (let attempt = 0; attempt <= config.maxRetries; attempt++) {
    try {
      return await fn();
    } catch (error) {
      lastError = error;

      if (attempt === config.maxRetries || !isTransientError(error)) {
        throw error;
      }

      const delay = Math.min(
        config.baseDelayMs * Math.pow(2, attempt),
        config.maxDelayMs,
      );
      console.warn(
        `Transient error (attempt ${attempt + 1}/${config.maxRetries + 1}), ` +
        `retrying in ${delay}ms: ${error instanceof Error ? error.message : error}`
      );
      await sleep(delay);
    }
  }

  throw lastError;
}

/**
 * Polyguard Trading Agent
 *
 * Manages automated trading on Polyguard prediction markets
 */
export class TradingAgent {
  private strategy: Strategy | null = null;
  private riskManager: RiskManager | null = null;
  private positionTracker: PositionTracker = new PositionTracker();
  private isRunning = false;
  private pollInterval: NodeJS.Timeout | null = null;

  constructor(
    private readonly connection: Connection,
    private readonly programId: PublicKey,
    private readonly agentPubkey: PublicKey,
    private readonly delegateKeypair: Keypair,
  ) {}

  /**
   * Load agent account from chain with retry
   */
  async loadAgent(): Promise<TradingAgentAccount> {
    const accountInfo = await withRetry(
      () => this.connection.getAccountInfo(this.agentPubkey),
    );
    if (!accountInfo) {
      throw new Error('Agent account not found');
    }

    // Parse account data (simplified - would use anchor deserialization)
    return this.deserializeAgent(accountInfo.data);
  }

  /**
   * Set trading strategy
   */
  setStrategy(strategy: Strategy): void {
    this.strategy = strategy;
  }

  /**
   * Start automated trading
   */
  async start(
    markets: PublicKey[],
    pollIntervalMs: number = 5000,
  ): Promise<void> {
    if (!this.strategy) {
      throw new Error('No strategy set');
    }

    const agentData = await this.loadAgent();
    this.riskManager = new RiskManager(agentData);

    if (agentData.status !== AgentStatus.Active) {
      throw new Error('Agent is not active');
    }

    this.isRunning = true;

    this.pollInterval = setInterval(async () => {
      if (!this.isRunning) return;

      for (const market of markets) {
        try {
          await this.processMarket(market);
        } catch (error) {
          console.error(`Error processing market ${market.toBase58()}:`, error);
        }
      }
    }, pollIntervalMs);

    console.log(`Agent started with strategy: ${this.strategy.name}`);
  }

  /**
   * Stop automated trading
   */
  stop(): void {
    this.isRunning = false;
    if (this.pollInterval) {
      clearInterval(this.pollInterval);
      this.pollInterval = null;
    }
    console.log('Agent stopped');
  }

  /**
   * Process a single market
   */
  private async processMarket(market: PublicKey): Promise<void> {
    if (!this.strategy || !this.riskManager) return;

    // Fetch market data
    const marketData = await this.fetchMarketData(market);

    // Analyze with strategy
    const signal = this.strategy.analyze(market, marketData);
    if (!signal) return;

    // Convert to order
    const agentData = await this.loadAgent();
    const order = this.strategy.toOrder(signal, agentData.availableBalance);
    if (!order) return;

    // Validate with risk manager
    const validation = this.riskManager.validateTrade(order);
    if (!validation.valid) {
      console.log(`Trade rejected: ${validation.failedChecks.map(c => c.message).join(', ')}`);
      return;
    }

    // Execute trade
    const result = await this.executeTrade(market, order);
    if (result.success) {
      console.log(`Trade executed: ${result.txId}`);

      // Track position
      this.positionTracker.addPosition({
        market,
        outcome: order.outcome,
        quantity: result.filledQuantity || order.quantity,
        avgEntryPrice: result.avgPrice || order.price,
        openedAt: Date.now(),
      });
    }
  }

  /**
   * Fetch market data with retry
   */
  private async fetchMarketData(market: PublicKey): Promise<MarketData> {
    const accountInfo = await withRetry(
      () => this.connection.getAccountInfo(market),
    );
    if (!accountInfo) {
      throw new Error('Market not found');
    }

    // Placeholder - would parse actual market data
    return {
      market,
      yesPrice: 5000,
      noPrice: 5000,
      volume24h: 0n,
      liquidity: 0n,
      lastUpdate: Date.now(),
    };
  }

  /**
   * Execute a trade with retry logic for transient errors
   */
  async executeTrade(market: PublicKey, order: OrderParams): Promise<TradeResult> {
    try {
      const txId = await withRetry(async () => {
        // Build fresh transaction (blockhash may have changed)
        const tx = await this.buildPlaceOrderTx(market, order);

        // Sign and send
        tx.sign(this.delegateKeypair);
        const signature = await this.connection.sendTransaction(
          tx,
          [this.delegateKeypair],
          { skipPreflight: false },
        );

        // Confirm with retry
        const confirmation = await withRetry(
          () => this.connection.confirmTransaction(signature, 'confirmed'),
          { maxRetries: 2, baseDelayMs: 1000, maxDelayMs: 5000 },
        );

        if (confirmation.value.err) {
          throw new Error(`Transaction failed: ${JSON.stringify(confirmation.value.err)}`);
        }

        return signature;
      });

      return {
        success: true,
        txId,
        filledQuantity: order.quantity,
        avgPrice: order.price,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }

  /**
   * Build place order transaction
   */
  private async buildPlaceOrderTx(
    market: PublicKey,
    order: OrderParams,
  ): Promise<Transaction> {
    // Simplified - would use actual program instruction
    const ix = new TransactionInstruction({
      keys: [
        { pubkey: this.delegateKeypair.publicKey, isSigner: true, isWritable: true },
        { pubkey: this.agentPubkey, isSigner: false, isWritable: true },
        { pubkey: market, isSigner: false, isWritable: true },
      ],
      programId: this.programId,
      data: Buffer.from([
        0, // instruction index for agent_place_order
        order.side,
        order.outcome,
        ...new BN(order.price).toArray('le', 8),
        ...new BN(order.quantity.toString()).toArray('le', 8),
        order.orderType,
        ...new BN((order.clientOrderId || 0n).toString()).toArray('le', 8),
      ]),
    });

    const tx = new Transaction().add(ix);
    tx.recentBlockhash = (await this.connection.getLatestBlockhash()).blockhash;
    tx.feePayer = this.delegateKeypair.publicKey;

    return tx;
  }

  /**
   * Get agent metrics
   */
  async getMetrics(): Promise<AgentMetrics> {
    const agentData = await this.loadAgent();

    const winRate = agentData.tradesCount > 0n
      ? Number(agentData.winCount) / Number(agentData.tradesCount)
      : 0;

    const avgPnl = agentData.tradesCount > 0n
      ? agentData.totalPnl / agentData.tradesCount
      : 0n;

    const maxDrawdownBps = agentData.highWaterMark > 0n
      ? Number((agentData.currentDrawdown * 10000n) / agentData.highWaterMark)
      : 0;

    return {
      totalPnl: agentData.totalPnl,
      winRate,
      tradesCount: agentData.tradesCount,
      avgPnlPerTrade: avgPnl,
      maxDrawdown: maxDrawdownBps,
      volumeTraded: agentData.volumeTraded,
    };
  }

  /**
   * Get open positions
   */
  getPositions(): Position[] {
    return this.positionTracker.getAllPositions();
  }

  /**
   * Deserialize agent account (simplified)
   */
  private deserializeAgent(data: Buffer): TradingAgentAccount {
    // Simplified deserialization - would use anchor
    // This is a placeholder that returns mock data
    return {
      owner: new PublicKey(data.slice(8, 40)),
      delegate: new PublicKey(data.slice(40, 72)),
      name: 'Agent',
      bump: data[72],
      status: data[73] as AgentStatus,
      version: data[74],
      maxPositionSize: BigInt(10000),
      maxTotalExposure: BigInt(100000),
      riskParams: {
        maxDrawdownBps: 2000,
        maxDailyLoss: BigInt(5000),
        minEdgeBps: 100,
        positionSizing: 0,
        sizingParam: BigInt(1000),
      },
      totalDeposited: BigInt(100000),
      availableBalance: BigInt(100000),
      lockedBalance: BigInt(0),
      totalPnl: BigInt(0),
      highWaterMark: BigInt(100000),
      currentDrawdown: BigInt(0),
      dailyLoss: BigInt(0),
      lastDay: BigInt(0),
      activePositions: 0,
      tradesCount: BigInt(0),
      winCount: BigInt(0),
      volumeTraded: BigInt(0),
      createdAt: BigInt(Date.now()),
      lastTradeAt: BigInt(0),
      allowedMarketsCount: 0,
      allowedMarkets: [],
    };
  }
}

/**
 * Create a new trading agent account with retry logic
 */
export async function createAgent(
  connection: Connection,
  programId: PublicKey,
  owner: Keypair,
  params: CreateAgentParams,
): Promise<PublicKey> {
  // Find PDA
  const [agentPda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from('trading_agent'),
      owner.publicKey.toBuffer(),
      Buffer.from(params.name),
    ],
    programId,
  );

  await withRetry(async () => {
    // Build fresh transaction
    const ix = new TransactionInstruction({
      keys: [
        { pubkey: owner.publicKey, isSigner: true, isWritable: true },
        { pubkey: agentPda, isSigner: false, isWritable: true },
      ],
      programId,
      data: Buffer.from([1]), // create_agent instruction
    });

    const tx = new Transaction().add(ix);
    tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;
    tx.feePayer = owner.publicKey;
    tx.sign(owner);

    const signature = await connection.sendTransaction(tx, [owner]);

    // Confirm the transaction
    const confirmation = await withRetry(
      () => connection.confirmTransaction(signature, 'confirmed'),
      { maxRetries: 2, baseDelayMs: 1000, maxDelayMs: 5000 },
    );

    if (confirmation.value.err) {
      throw new Error(`Create agent failed: ${JSON.stringify(confirmation.value.err)}`);
    }
  });

  return agentPda;
}
