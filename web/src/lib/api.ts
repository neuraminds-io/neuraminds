import type {
  Market,
  Order,
  Position,
  OrderBook,
  Trade,
  User,
  Transaction,
  PlaceOrderRequest,
  PlaceOrderResponse,
  CancelOrderResponse,
  ClaimWinningsResponse,
  PaginatedResponse,
  MarketFilters,
  OrderFilters,
  Outcome,
  WalletBalance,
  DepositAddress,
  DepositRequest,
  DepositResponse,
  WithdrawRequest,
  WithdrawResponse,
  Notification,
  NotificationPreferences,
  Leaderboard,
  LeaderboardPeriod,
  LeaderboardMetric,
  PublicProfile,
  ProfileActivity,
} from '@/types';
import { CURATED_MARKETS_BY_ID } from '@/lib/curatedMarkets';

const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080/v1';

export interface BaseTokenState {
  chain_id: number;
  token_address: string;
  total_supply_hex: string;
  decimals: number;
}

interface BaseMarketSnapshot {
  id: string;
  question_hash: string;
  question: string;
  description: string;
  category: string;
  resolution_source: string;
  resolver: string;
  close_time: number;
  resolve_time: number;
  resolved: boolean;
  outcome?: 'yes' | 'no' | null;
  status: string;
}

interface BaseMarketsResponse {
  markets: BaseMarketSnapshot[];
  total: number;
  limit: number;
  offset: number;
}

interface BaseOrderBookLevel {
  price: number;
  quantity: number;
  orders: number;
}

interface BaseOrderBookResponse {
  market_id: string;
  outcome: 'yes' | 'no';
  bids: BaseOrderBookLevel[];
  asks: BaseOrderBookLevel[];
  last_updated: string;
}

interface BaseTradeSnapshot {
  id: string;
  market_id: string;
  outcome: 'yes' | 'no';
  price: number;
  price_bps: number;
  quantity: number;
  tx_hash: string;
  block_number: number;
  created_at: string;
}

interface BaseTradesResponse {
  trades: BaseTradeSnapshot[];
  total: number;
  limit: number;
  offset: number;
  has_more: boolean;
}

export interface PreparedEvmWriteTx {
  chain_id: number;
  from?: string;
  to: string;
  data: `0x${string}`;
  value: `0x${string}`;
  method: string;
}

export interface RelayRawTxResponse {
  chain_id: number;
  tx_hash: string;
}

class ApiError extends Error {
  constructor(public status: number, message: string) {
    super(message);
    this.name = 'ApiError';
  }
}

function toNumber(value: unknown, fallback = 0): number {
  if (typeof value === 'number' && Number.isFinite(value)) return value;
  if (typeof value === 'string') {
    const parsed = Number(value);
    if (Number.isFinite(parsed)) return parsed;
  }
  return fallback;
}

function toIsoString(value: unknown): string {
  if (typeof value === 'string' && value.length > 0) return value;
  return new Date().toISOString();
}

function fromUnixSeconds(value: number | undefined): string {
  if (!value || !Number.isFinite(value) || value <= 0) {
    return new Date().toISOString();
  }
  return new Date(value * 1000).toISOString();
}

function normalizeMarketStatus(value: unknown): Market['status'] {
  if (
    value === 'active' ||
    value === 'paused' ||
    value === 'closed' ||
    value === 'resolved' ||
    value === 'cancelled'
  ) {
    return value;
  }
  return 'active';
}

function normalizeMarket(raw: Record<string, unknown>): Market {
  const yesPrice = toNumber(raw.yesPrice ?? raw.yes_price, 0.5);
  const noPrice = toNumber(raw.noPrice ?? raw.no_price, 1 - yesPrice);

  return {
    id: String(raw.id ?? ''),
    address: String(raw.address ?? raw.id ?? ''),
    question: String(raw.question ?? ''),
    description: String(raw.description ?? ''),
    category: String(raw.category ?? 'unknown'),
    status: normalizeMarketStatus(raw.status),
    yesPrice,
    noPrice,
    yesSupply: toNumber(raw.yesSupply ?? raw.yes_supply),
    noSupply: toNumber(raw.noSupply ?? raw.no_supply),
    volume24h: toNumber(raw.volume24h ?? raw.volume_24h),
    totalVolume: toNumber(raw.totalVolume ?? raw.total_volume),
    totalCollateral: toNumber(raw.totalCollateral ?? raw.total_collateral),
    feeBps: toNumber(raw.feeBps ?? raw.fee_bps),
    oracle: String(raw.oracle ?? ''),
    collateralMint: String(raw.collateralMint ?? raw.collateral_mint ?? ''),
    yesMint: String(raw.yesMint ?? raw.yes_mint ?? ''),
    noMint: String(raw.noMint ?? raw.no_mint ?? ''),
    resolutionDeadline: toIsoString(raw.resolutionDeadline ?? raw.resolution_deadline),
    tradingEnd: toIsoString(raw.tradingEnd ?? raw.trading_end),
    resolvedOutcome: (raw.resolvedOutcome ?? raw.resolved_outcome) as Market['resolvedOutcome'],
    createdAt: toIsoString(raw.createdAt ?? raw.created_at),
    resolvedAt: raw.resolvedAt || raw.resolved_at ? toIsoString(raw.resolvedAt ?? raw.resolved_at) : undefined,
  };
}

function mapBaseSnapshotToMarket(snapshot: BaseMarketSnapshot): Market {
  const curated = CURATED_MARKETS_BY_ID[Number(snapshot.id)];
  const resolvedOutcome = snapshot.outcome === 'yes' || snapshot.outcome === 'no'
    ? snapshot.outcome
    : undefined;

  const yesPrice = resolvedOutcome === 'yes' ? 1 : resolvedOutcome === 'no' ? 0 : 0.5;
  const noPrice = 1 - yesPrice;

  const tradingEnd = fromUnixSeconds(snapshot.close_time);
  const resolutionDeadline = fromUnixSeconds(snapshot.resolve_time || snapshot.close_time);
  const question = snapshot.question?.trim() || curated?.question || `Base market #${snapshot.id}`;
  const description = snapshot.description?.trim()
    || (curated ? `Outcomes: ${curated.outcomes}. Context: ${curated.rationale}` : `Question hash: ${snapshot.question_hash}`);
  const category = snapshot.category?.trim() || curated?.category || 'base';

  return {
    id: snapshot.id,
    address: `base-market-${snapshot.id}`,
    question,
    description,
    category,
    status: normalizeMarketStatus(snapshot.status),
    yesPrice,
    noPrice,
    yesSupply: 0,
    noSupply: 0,
    volume24h: 0,
    totalVolume: 0,
    totalCollateral: 0,
    feeBps: 0,
    oracle: snapshot.resolver,
    collateralMint: '',
    yesMint: '',
    noMint: '',
    resolutionDeadline,
    tradingEnd,
    resolvedOutcome,
    createdAt: tradingEnd,
    resolvedAt: snapshot.resolved ? fromUnixSeconds(snapshot.resolve_time) : undefined,
  };
}

function normalizeOutcome(value: unknown): Outcome {
  return value === 'no' ? 'no' : 'yes';
}

function normalizeTrade(raw: Record<string, unknown>): Trade {
  return {
    id: String(raw.id ?? ''),
    marketId: String(raw.marketId ?? raw.market_id ?? ''),
    outcome: normalizeOutcome(raw.outcome),
    price: toNumber(raw.price),
    quantity: toNumber(raw.quantity),
    buyer: String(raw.buyer ?? ''),
    seller: String(raw.seller ?? ''),
    txSignature: String(raw.txSignature ?? raw.tx_signature ?? raw.tx_hash ?? ''),
    createdAt: toIsoString(raw.createdAt ?? raw.created_at),
  };
}

function mapBaseTradeToTrade(snapshot: BaseTradeSnapshot): Trade {
  return {
    id: snapshot.id,
    marketId: snapshot.market_id,
    outcome: snapshot.outcome,
    price: snapshot.price,
    quantity: snapshot.quantity,
    buyer: '',
    seller: '',
    txSignature: snapshot.tx_hash,
    createdAt: snapshot.created_at,
  };
}

// Access token stored in memory only (XSS-safe)
// Refresh token stored in httpOnly cookie (handled by /api/auth)
class ApiClient {
  private accessToken: string | null = null;
  private tokenExpiresAt: number | null = null;
  private refreshPromise: Promise<void> | null = null;

  setAccessToken(accessToken: string, expiresAt?: number) {
    this.accessToken = accessToken;
    this.tokenExpiresAt = expiresAt || Date.now() + 15 * 60 * 1000; // Default 15 min
  }

  clearAccessToken() {
    this.accessToken = null;
    this.tokenExpiresAt = null;
  }

  isAuthenticated(): boolean {
    return !!this.accessToken;
  }

  isTokenExpiringSoon(): boolean {
    if (!this.tokenExpiresAt) return true;
    // Refresh if less than 1 minute remaining
    return Date.now() > this.tokenExpiresAt - 60 * 1000;
  }

  // Check if we have a refresh token (httpOnly cookie)
  async checkSession(): Promise<boolean> {
    try {
      const res = await fetch('/api/auth', { method: 'GET' });
      const data = await res.json();
      return data.hasRefreshToken;
    } catch {
      return false;
    }
  }

  private async request<T>(
    path: string,
    options: RequestInit = {},
    skipRefresh = false
  ): Promise<T> {
    // Auto-refresh token if expiring soon
    if (!skipRefresh && this.accessToken && this.isTokenExpiringSoon()) {
      await this.refreshSession();
    }

    const headers: HeadersInit = {
      'Content-Type': 'application/json',
      ...options.headers,
    };

    if (this.accessToken) {
      (headers as Record<string, string>)['Authorization'] = `Bearer ${this.accessToken}`;
    }

    const res = await fetch(`${API_BASE}${path}`, {
      ...options,
      headers,
    });

    // Handle 401 by attempting token refresh
    if (res.status === 401 && !skipRefresh && this.accessToken) {
      try {
        await this.refreshSession();
        // Retry request with new token
        return this.request(path, options, true);
      } catch {
        this.clearAccessToken();
        throw new ApiError(401, 'Session expired');
      }
    }

    if (!res.ok) {
      const text = await res.text();
      throw new ApiError(res.status, text || res.statusText);
    }

    if (res.status === 204) {
      return {} as T;
    }

    return res.json();
  }

  // Refresh access token using httpOnly cookie
  private async refreshSession(): Promise<void> {
    // Prevent concurrent refresh calls
    if (this.refreshPromise) {
      return this.refreshPromise;
    }

    this.refreshPromise = (async () => {
      try {
        const res = await fetch('/api/auth', { method: 'PUT' });
        if (!res.ok) {
          this.clearAccessToken();
          throw new Error('Refresh failed');
        }
        const data = await res.json();
        this.setAccessToken(data.accessToken, data.expiresAt);
      } finally {
        this.refreshPromise = null;
      }
    })();

    return this.refreshPromise;
  }

  private buildQuery(params: Record<string, unknown> | object): string {
    const filtered = Object.entries(params).filter(
      ([, v]) => v !== undefined && v !== null
    );
    if (filtered.length === 0) return '';
    return '?' + new URLSearchParams(
      filtered.map(([k, v]) => [k, String(v)])
    ).toString();
  }

  // Markets
  async getMarkets(filters?: MarketFilters): Promise<PaginatedResponse<Market>> {
    const query = this.buildQuery(filters || {});
    const response = await this.request<{
      markets?: Record<string, unknown>[];
      data?: Record<string, unknown>[];
      total?: number;
      limit?: number;
      offset?: number;
      hasMore?: boolean;
    }>(`/markets${query}`);

    const marketsRaw = response.markets ?? response.data ?? [];
    const data = marketsRaw.map((market) => normalizeMarket(market));
    const total = toNumber(response.total, data.length);
    const limit = toNumber(response.limit, data.length);
    const offset = toNumber(response.offset, 0);

    return {
      data,
      total,
      limit,
      offset,
      hasMore: response.hasMore ?? offset + limit < total,
    };
  }

  async getMarket(id: string): Promise<Market> {
    const response = await this.request<Record<string, unknown>>(`/markets/${id}`);
    return normalizeMarket(response);
  }

  async getOrderBook(
    marketId: string,
    outcome: Outcome,
    depth = 20
  ): Promise<OrderBook> {
    const response = await this.request<{
      marketId?: string;
      market_id?: string;
      outcome?: Outcome;
      bids?: BaseOrderBookLevel[];
      asks?: BaseOrderBookLevel[];
      timestamp?: string;
      lastUpdated?: string;
      last_updated?: string;
    }>(
      `/markets/${marketId}/orderbook?outcome=${outcome}&depth=${depth}`
    );

    return {
      marketId: String(response.marketId ?? response.market_id ?? marketId),
      outcome: response.outcome === 'no' ? 'no' : 'yes',
      bids: response.bids ?? [],
      asks: response.asks ?? [],
      lastUpdated: toIsoString(response.lastUpdated ?? response.last_updated ?? response.timestamp),
    };
  }

  async getTrades(
    marketId: string,
    params?: { outcome?: Outcome; limit?: number; before?: string }
  ): Promise<PaginatedResponse<Trade>> {
    return this.getBaseTrades(marketId, params);
  }

  // Orders
  async getOrders(filters?: OrderFilters): Promise<PaginatedResponse<Order>> {
    const query = this.buildQuery(filters || {});
    return this.request(`/orders${query}`);
  }

  async getOrder(orderId: string): Promise<Order> {
    return this.request(`/orders/${orderId}`);
  }

  async placeOrder(data: PlaceOrderRequest): Promise<PlaceOrderResponse> {
    return this.request('/orders', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async cancelOrder(orderId: string): Promise<CancelOrderResponse> {
    return this.request(`/orders/${orderId}`, {
      method: 'DELETE',
    });
  }

  // Positions
  async getPositions(): Promise<PaginatedResponse<Position>> {
    return this.request<PaginatedResponse<Position>>('/positions');
  }

  async getPosition(marketId: string): Promise<Position> {
    return this.request(`/positions/${marketId}`);
  }

  async claimWinnings(marketId: string): Promise<ClaimWinningsResponse> {
    return this.request(`/positions/${marketId}/claim`, {
      method: 'POST',
    });
  }

  // User
  async getProfile(): Promise<User> {
    return this.request('/user/profile');
  }

  async getTransactions(params?: {
    limit?: number;
    offset?: number;
    txType?: string;
  }): Promise<PaginatedResponse<Transaction>> {
    const query = this.buildQuery(params || {});
    return this.request(`/user/transactions${query}`);
  }

  // Wallet
  async getWalletBalance(): Promise<WalletBalance> {
    return this.request('/wallet/balance');
  }

  async getBaseMarkets(params?: { limit?: number; offset?: number }): Promise<PaginatedResponse<Market>> {
    const query = this.buildQuery(params || {});
    const response = await this.request<BaseMarketsResponse>(`/evm/markets${query}`);
    const data = response.markets.map(mapBaseSnapshotToMarket);
    const total = toNumber(response.total, data.length);
    const limit = toNumber(response.limit, data.length);
    const offset = toNumber(response.offset, 0);

    return {
      data,
      total,
      limit,
      offset,
      hasMore: offset + limit < total,
    };
  }

  async getBaseOrderBook(
    marketId: string,
    outcome: Outcome,
    depth = 20
  ): Promise<OrderBook> {
    const query = this.buildQuery({ outcome, depth });
    const response = await this.request<BaseOrderBookResponse>(
      `/evm/markets/${marketId}/orderbook${query}`
    );

    return {
      marketId: response.market_id,
      outcome: response.outcome,
      bids: response.bids ?? [],
      asks: response.asks ?? [],
      lastUpdated: toIsoString(response.last_updated),
    };
  }

  async getBaseTrades(
    marketId: string,
    params?: { outcome?: Outcome; limit?: number; before?: string; offset?: number }
  ): Promise<PaginatedResponse<Trade>> {
    const query = this.buildQuery({
      outcome: params?.outcome,
      limit: params?.limit,
      offset: params?.offset,
    });
    const response = await this.request<BaseTradesResponse>(
      `/evm/markets/${marketId}/trades${query}`
    );
    const data = (response.trades ?? []).map(mapBaseTradeToTrade);
    const total = toNumber(response.total, data.length);
    const limit = toNumber(response.limit, params?.limit ?? data.length);
    const offset = toNumber(response.offset, params?.offset ?? 0);

    return {
      data,
      total,
      limit,
      offset,
      hasMore: response.has_more ?? offset + limit < total,
    };
  }

  async getBaseMarket(id: string): Promise<Market> {
    const parsedId = Number(id);
    if (!Number.isInteger(parsedId) || parsedId < 1) {
      throw new ApiError(404, 'Market not found');
    }

    const page = await this.getBaseMarkets({ limit: 1, offset: parsedId - 1 });
    const market = page.data[0];
    if (!market || market.id !== id) {
      throw new ApiError(404, 'Market not found');
    }

    return market;
  }

  async getBaseTokenState(): Promise<BaseTokenState> {
    return this.request('/evm/token/state');
  }

  async prepareBaseCreateMarket(data: {
    from?: string;
    question: string;
    description?: string;
    category?: string;
    resolutionSource?: string;
    closeTime: number;
    resolver: string;
  }): Promise<PreparedEvmWriteTx> {
    return this.request('/evm/write/markets/create', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async prepareBasePlaceOrder(data: {
    from?: string;
    marketId: number;
    outcome: Outcome;
    priceBps: number;
    size: string;
    expiry: number;
  }): Promise<PreparedEvmWriteTx> {
    return this.request('/evm/write/orders/place', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async prepareBaseCancelOrder(data: {
    from?: string;
    orderId: number;
  }): Promise<PreparedEvmWriteTx> {
    return this.request('/evm/write/orders/cancel', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async prepareBaseClaim(data: {
    from?: string;
    marketId: number;
  }): Promise<PreparedEvmWriteTx> {
    return this.request('/evm/write/positions/claim', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async prepareBaseMatchOrders(data: {
    from?: string;
    firstOrderId: number;
    secondOrderId: number;
    fillSize: string;
  }): Promise<PreparedEvmWriteTx> {
    return this.request('/evm/write/orders/match', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async prepareBaseCreateAgent(data: {
    from?: string;
    marketId: number;
    isYes: boolean;
    priceBps: number;
    size: string;
    cadence: number;
    expiryWindow: number;
    strategy: string;
  }): Promise<PreparedEvmWriteTx> {
    return this.request('/evm/write/agents/create', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async prepareBaseExecuteAgent(data: {
    from?: string;
    agentId: number;
  }): Promise<PreparedEvmWriteTx> {
    return this.request('/evm/write/agents/execute', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async relayBaseRawTransaction(rawTx: string): Promise<RelayRawTxResponse> {
    return this.request('/evm/write/relay', {
      method: 'POST',
      body: JSON.stringify({ rawTx }),
    });
  }

  async getDepositAddress(): Promise<DepositAddress> {
    return this.request('/wallet/deposit/address');
  }

  async deposit(data: DepositRequest): Promise<DepositResponse> {
    return this.request('/wallet/deposit', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async withdraw(data: WithdrawRequest): Promise<WithdrawResponse> {
    return this.request('/wallet/withdraw', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  // Auth
  async getNonce(): Promise<string> {
    return this.getSiweNonce();
  }

  async getSiweNonce(): Promise<string> {
    const res = await this.request<{ nonce: string }>('/auth/siwe/nonce', {}, true);
    return res.nonce;
  }

  async login(
    wallet: string,
    signature: string,
    message: string
  ): Promise<{ accessToken: string; expiresAt: number }> {
    return this.loginSiwe(wallet, signature, message);
  }

  async loginSiwe(
    wallet: string,
    signature: string,
    message: string
  ): Promise<{ accessToken: string; expiresAt: number }> {
    const res = await fetch('/api/auth', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ wallet, signature, message, flow: 'siwe' }),
    });

    if (!res.ok) {
      const data = await res.json();
      throw new ApiError(res.status, data.error || 'SIWE login failed');
    }

    const data = await res.json();
    this.setAccessToken(data.accessToken, data.expiresAt);
    return data;
  }

  async refresh(): Promise<{ accessToken: string; expiresAt: number }> {
    const res = await fetch('/api/auth', { method: 'PUT' });

    if (!res.ok) {
      this.clearAccessToken();
      throw new ApiError(res.status, 'Token refresh failed');
    }

    const data = await res.json();
    this.setAccessToken(data.accessToken, data.expiresAt);
    return data;
  }

  async logout(): Promise<void> {
    try {
      await fetch('/api/auth', { method: 'DELETE' });
    } finally {
      this.clearAccessToken();
    }
  }

  // Restore session on page load (if refresh token exists)
  async restoreSession(): Promise<boolean> {
    const hasToken = await this.checkSession();
    if (hasToken) {
      try {
        await this.refresh();
        return true;
      } catch {
        return false;
      }
    }
    return false;
  }

  // Notifications
  async getNotifications(params?: {
    limit?: number;
    offset?: number;
    unreadOnly?: boolean;
  }): Promise<PaginatedResponse<Notification>> {
    const query = this.buildQuery(params || {});
    return this.request(`/notifications${query}`);
  }

  async getUnreadCount(): Promise<{ count: number }> {
    return this.request('/notifications/unread-count');
  }

  async markAsRead(notificationId: string): Promise<void> {
    return this.request(`/notifications/${notificationId}/read`, {
      method: 'PUT',
    });
  }

  async markAllAsRead(): Promise<void> {
    return this.request('/notifications/read-all', {
      method: 'PUT',
    });
  }

  async getNotificationPreferences(): Promise<NotificationPreferences> {
    return this.request('/notifications/preferences');
  }

  async updateNotificationPreferences(
    prefs: Partial<NotificationPreferences>
  ): Promise<NotificationPreferences> {
    return this.request('/notifications/preferences', {
      method: 'PUT',
      body: JSON.stringify(prefs),
    });
  }

  // Leaderboards
  async getLeaderboard(
    period: LeaderboardPeriod = 'weekly',
    metric: LeaderboardMetric = 'pnl',
    limit = 100
  ): Promise<Leaderboard> {
    return this.request(`/leaderboard?period=${period}&metric=${metric}&limit=${limit}`);
  }

  async getUserRank(
    wallet: string,
    period: LeaderboardPeriod = 'weekly',
    metric: LeaderboardMetric = 'pnl'
  ): Promise<{ rank: number; value: number }> {
    return this.request(`/leaderboard/rank/${wallet}?period=${period}&metric=${metric}`);
  }

  // Public profiles
  async getPublicProfile(wallet: string): Promise<PublicProfile> {
    return this.request(`/profiles/${wallet}`);
  }

  async getProfileActivity(
    wallet: string,
    params?: { limit?: number; offset?: number }
  ): Promise<PaginatedResponse<ProfileActivity>> {
    const query = this.buildQuery(params || {});
    return this.request(`/profiles/${wallet}/activity${query}`);
  }

  async getProfilePositions(wallet: string): Promise<PaginatedResponse<Position>> {
    return this.request(`/profiles/${wallet}/positions`);
  }
}

export const api = new ApiClient();
export { ApiError };
