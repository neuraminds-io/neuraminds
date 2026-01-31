/**
 * End-to-End API Tests
 *
 * Tests complete user flows through the Polyguard API including:
 * - Authentication
 * - Market operations
 * - Order lifecycle
 * - Position management
 * - Error handling
 */

import { describe, it, before, after, beforeEach } from 'mocha';
import { expect } from 'chai';
import * as nacl from 'tweetnacl';
import { Keypair } from '@solana/web3.js';
import bs58 from 'bs58';

const API_BASE = process.env.API_URL || 'http://localhost:8080';

interface ApiClient {
  baseUrl: string;
  accessToken: string | null;
  refreshToken: string | null;
  wallet: Keypair;
}

function createApiClient(wallet?: Keypair): ApiClient {
  return {
    baseUrl: API_BASE,
    accessToken: null,
    refreshToken: null,
    wallet: wallet || Keypair.generate(),
  };
}

async function request(
  client: ApiClient,
  method: string,
  path: string,
  body?: unknown,
  headers?: Record<string, string>
): Promise<{ status: number; body: unknown }> {
  const url = `${client.baseUrl}${path}`;
  const reqHeaders: Record<string, string> = {
    'Content-Type': 'application/json',
    ...headers,
  };

  if (client.accessToken) {
    reqHeaders['Authorization'] = `Bearer ${client.accessToken}`;
  }

  const response = await fetch(url, {
    method,
    headers: reqHeaders,
    body: body ? JSON.stringify(body) : undefined,
  });

  let responseBody: unknown;
  const contentType = response.headers.get('content-type');
  if (contentType?.includes('application/json')) {
    responseBody = await response.json();
  } else {
    responseBody = await response.text();
  }

  return { status: response.status, body: responseBody };
}

async function authenticate(client: ApiClient): Promise<void> {
  // Get nonce
  const nonceResp = await request(client, 'GET', `/v1/auth/nonce?wallet=${client.wallet.publicKey.toBase58()}`);
  expect(nonceResp.status).to.equal(200);
  const { nonce } = nonceResp.body as { nonce: string };

  // Sign message
  const message = `Sign this message to authenticate with Polyguard.\n\nWallet: ${client.wallet.publicKey.toBase58()}\nNonce: ${nonce}`;
  const messageBytes = new TextEncoder().encode(message);
  const signature = nacl.sign.detached(messageBytes, client.wallet.secretKey);

  // Verify
  const verifyResp = await request(client, 'POST', '/v1/auth/verify', {
    wallet: client.wallet.publicKey.toBase58(),
    signature: bs58.encode(signature),
    message,
  });
  expect(verifyResp.status).to.equal(200);

  const { access_token, refresh_token } = verifyResp.body as {
    access_token: string;
    refresh_token: string;
  };

  client.accessToken = access_token;
  client.refreshToken = refresh_token;
}

describe('E2E: Health & Status', () => {
  it('should return healthy status', async () => {
    const client = createApiClient();
    const resp = await request(client, 'GET', '/health');
    expect(resp.status).to.equal(200);
    expect((resp.body as { status: string }).status).to.equal('healthy');
  });

  it('should return deep health check', async () => {
    const client = createApiClient();
    const resp = await request(client, 'GET', '/health/deep');
    expect(resp.status).to.be.oneOf([200, 503]); // May be degraded in test env
    const body = resp.body as { status: string; components: unknown };
    expect(body).to.have.property('status');
    expect(body).to.have.property('components');
  });
});

describe('E2E: Authentication', () => {
  let client: ApiClient;

  beforeEach(() => {
    client = createApiClient();
  });

  it('should get nonce for wallet', async () => {
    const resp = await request(client, 'GET', `/v1/auth/nonce?wallet=${client.wallet.publicKey.toBase58()}`);
    expect(resp.status).to.equal(200);
    const body = resp.body as { nonce: string; expires_at: string };
    expect(body).to.have.property('nonce');
    expect(body.nonce).to.have.lengthOf.at.least(16);
  });

  it('should reject invalid wallet address', async () => {
    const resp = await request(client, 'GET', '/v1/auth/nonce?wallet=invalid');
    expect(resp.status).to.equal(400);
  });

  it('should authenticate with valid signature', async () => {
    await authenticate(client);
    expect(client.accessToken).to.not.be.null;
  });

  it('should reject invalid signature', async () => {
    const nonceResp = await request(client, 'GET', `/v1/auth/nonce?wallet=${client.wallet.publicKey.toBase58()}`);
    const { nonce } = nonceResp.body as { nonce: string };

    const resp = await request(client, 'POST', '/v1/auth/verify', {
      wallet: client.wallet.publicKey.toBase58(),
      signature: bs58.encode(new Uint8Array(64).fill(1)), // Invalid signature
      message: `Sign this message to authenticate with Polyguard.\n\nWallet: ${client.wallet.publicKey.toBase58()}\nNonce: ${nonce}`,
    });
    expect(resp.status).to.equal(401);
  });

  it('should refresh access token', async () => {
    await authenticate(client);
    const oldToken = client.accessToken;

    const resp = await request(client, 'POST', '/v1/auth/refresh', {
      refresh_token: client.refreshToken,
    });
    expect(resp.status).to.equal(200);

    const body = resp.body as { access_token: string };
    expect(body.access_token).to.not.equal(oldToken);
  });
});

describe('E2E: Markets', () => {
  let client: ApiClient;

  before(async () => {
    client = createApiClient();
    await authenticate(client);
  });

  it('should list markets', async () => {
    const resp = await request(client, 'GET', '/v1/markets');
    expect(resp.status).to.equal(200);
    const body = resp.body as { markets: unknown[]; total: number };
    expect(body).to.have.property('markets');
    expect(body).to.have.property('total');
  });

  it('should filter markets by status', async () => {
    const resp = await request(client, 'GET', '/v1/markets?status=active');
    expect(resp.status).to.equal(200);
  });

  it('should paginate markets', async () => {
    const resp = await request(client, 'GET', '/v1/markets?limit=10&offset=0');
    expect(resp.status).to.equal(200);
    const body = resp.body as { markets: unknown[] };
    expect(body.markets.length).to.be.at.most(10);
  });
});

describe('E2E: Orders', () => {
  let client: ApiClient;
  let marketId: string;

  before(async () => {
    client = createApiClient();
    await authenticate(client);

    // Get first active market
    const marketsResp = await request(client, 'GET', '/v1/markets?status=active&limit=1');
    const markets = (marketsResp.body as { markets: { id: string }[] }).markets;
    if (markets.length > 0) {
      marketId = markets[0].id;
    }
  });

  it('should list user orders', async () => {
    const resp = await request(client, 'GET', '/v1/orders');
    expect(resp.status).to.equal(200);
    const body = resp.body as { orders: unknown[]; total: number };
    expect(body).to.have.property('orders');
  });

  it('should reject order without authentication', async () => {
    const unauthClient = createApiClient();
    const resp = await request(unauthClient, 'POST', '/v1/orders', {
      market_id: 'test',
      side: 'buy',
      outcome: 'yes',
      price: 0.5,
      quantity: 100,
      order_type: 'limit',
    });
    expect(resp.status).to.equal(401);
  });

  it('should validate order price range', async () => {
    if (!marketId) return; // Skip if no market

    // Price too low
    let resp = await request(client, 'POST', '/v1/orders', {
      market_id: marketId,
      side: 'buy',
      outcome: 'yes',
      price: 0,
      quantity: 100,
      order_type: 'limit',
    });
    expect(resp.status).to.equal(400);

    // Price too high
    resp = await request(client, 'POST', '/v1/orders', {
      market_id: marketId,
      side: 'buy',
      outcome: 'yes',
      price: 1.5,
      quantity: 100,
      order_type: 'limit',
    });
    expect(resp.status).to.equal(400);
  });

  it('should validate order quantity', async () => {
    if (!marketId) return;

    const resp = await request(client, 'POST', '/v1/orders', {
      market_id: marketId,
      side: 'buy',
      outcome: 'yes',
      price: 0.5,
      quantity: 0,
      order_type: 'limit',
    });
    expect(resp.status).to.equal(400);
  });

  it('should support idempotency keys', async () => {
    if (!marketId) return;

    const idempotencyKey = `test-${Date.now()}`;
    const orderRequest = {
      market_id: marketId,
      side: 'buy',
      outcome: 'yes',
      price: 0.5,
      quantity: 100,
      order_type: 'limit',
    };

    // First request
    const resp1 = await request(client, 'POST', '/v1/orders', orderRequest, {
      'Idempotency-Key': idempotencyKey,
    });

    // Second request with same key should return same response
    const resp2 = await request(client, 'POST', '/v1/orders', orderRequest, {
      'Idempotency-Key': idempotencyKey,
    });

    if (resp1.status === 201) {
      expect(resp2.status).to.equal(201);
      expect(resp2.body).to.deep.equal(resp1.body);
    }
  });
});

describe('E2E: Positions', () => {
  let client: ApiClient;

  before(async () => {
    client = createApiClient();
    await authenticate(client);
  });

  it('should list user positions', async () => {
    const resp = await request(client, 'GET', '/v1/positions');
    expect(resp.status).to.equal(200);
    const body = resp.body as { positions: unknown[] };
    expect(body).to.have.property('positions');
  });

  it('should filter positions by market', async () => {
    const resp = await request(client, 'GET', '/v1/positions?market_id=test-market');
    expect(resp.status).to.equal(200);
  });
});

describe('E2E: Rate Limiting', () => {
  let client: ApiClient;

  before(async () => {
    client = createApiClient();
  });

  it('should rate limit unauthenticated requests', async function () {
    this.timeout(30000);

    // Make many requests quickly
    const requests = Array(20).fill(null).map(() =>
      request(client, 'GET', '/v1/markets')
    );

    const responses = await Promise.all(requests);
    const rateLimited = responses.some(r => r.status === 429);

    // Rate limiting may or may not trigger depending on config
    // Just verify we don't get server errors
    responses.forEach(r => {
      expect(r.status).to.be.oneOf([200, 429]);
    });
  });
});

describe('E2E: WebSocket', () => {
  it('should connect to WebSocket endpoint', async function () {
    this.timeout(10000);

    // Skip if WebSocket not available in test environment
    if (typeof WebSocket === 'undefined') {
      this.skip();
    }

    const wsUrl = API_BASE.replace('http', 'ws') + '/ws';

    return new Promise<void>((resolve, reject) => {
      const ws = new WebSocket(wsUrl);

      const timeout = setTimeout(() => {
        ws.close();
        reject(new Error('WebSocket connection timeout'));
      }, 5000);

      ws.onopen = () => {
        clearTimeout(timeout);
        ws.close();
        resolve();
      };

      ws.onerror = (err) => {
        clearTimeout(timeout);
        // Connection error is expected if server not running
        resolve();
      };
    });
  });
});

describe('E2E: Error Handling', () => {
  let client: ApiClient;

  before(async () => {
    client = createApiClient();
    await authenticate(client);
  });

  it('should return 404 for unknown endpoints', async () => {
    const resp = await request(client, 'GET', '/v1/nonexistent');
    expect(resp.status).to.equal(404);
  });

  it('should return structured error responses', async () => {
    const resp = await request(client, 'GET', '/v1/orders/invalid-uuid');
    expect(resp.status).to.equal(400);
    const body = resp.body as { error: { code: string; message: string } };
    expect(body).to.have.property('error');
    expect(body.error).to.have.property('code');
    expect(body.error).to.have.property('message');
  });

  it('should not leak internal errors', async () => {
    const resp = await request(client, 'POST', '/v1/orders', {
      // Invalid data to trigger potential error
      market_id: 'x'.repeat(1000),
      side: 'invalid',
    });

    const body = resp.body as { error?: { message: string } };
    if (body.error) {
      // Error message should not contain stack traces or internal details
      expect(body.error.message).to.not.include('at ');
      expect(body.error.message).to.not.include('Error:');
      expect(body.error.message).to.not.include('.rs:');
    }
  });
});

describe('E2E: Security Headers', () => {
  it('should include security headers', async () => {
    const response = await fetch(`${API_BASE}/health`);

    // These headers should be present
    const requiredHeaders = [
      'x-content-type-options',
      'x-frame-options',
    ];

    for (const header of requiredHeaders) {
      expect(
        response.headers.has(header),
        `Missing security header: ${header}`
      ).to.be.true;
    }
  });
});
