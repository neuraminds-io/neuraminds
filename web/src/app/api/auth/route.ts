import { NextRequest, NextResponse } from 'next/server';
import { cookies } from 'next/headers';

const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080/v1';
const IS_PRODUCTION = process.env.NODE_ENV === 'production';
const MAX_BODY_BYTES = 16 * 1024;
const RATE_LIMIT_WINDOW_MS = 60_000;
const RATE_LIMIT_MAX_REQUESTS = 30;
const RATE_LIMIT_BUCKETS = new Map<string, { count: number; resetAt: number }>();
const ALLOWED_ORIGINS = new Set(
  (process.env.AUTH_ALLOWED_ORIGINS || '')
    .split(',')
    .map((origin) => origin.trim())
    .filter(Boolean)
);

const REFRESH_TOKEN_COOKIE = 'neuraminds_refresh';
const COMPAT_REFRESH_TOKEN_COOKIES = ['neuralminds_refresh', 'polybit_refresh', 'polyguard_refresh'];
const COOKIE_MAX_AGE = 7 * 24 * 60 * 60;

function jsonError(status: number, error: string) {
  return NextResponse.json({ error }, { status });
}

function getClientIp(request: NextRequest): string {
  const forwardedFor = request.headers.get('x-forwarded-for');
  if (forwardedFor) {
    const firstIp = forwardedFor.split(',')[0]?.trim();
    if (firstIp) return firstIp;
  }

  const realIp = request.headers.get('x-real-ip')?.trim();
  if (realIp) return realIp;

  return 'unknown';
}

function cleanupRateLimitBuckets(now: number) {
  if (RATE_LIMIT_BUCKETS.size < 4_096) return;

  RATE_LIMIT_BUCKETS.forEach((bucket, key) => {
    if (bucket.resetAt <= now) {
      RATE_LIMIT_BUCKETS.delete(key);
    }
  });
}

function checkRateLimit(request: NextRequest): NextResponse | null {
  const now = Date.now();
  cleanupRateLimitBuckets(now);

  const key = getClientIp(request);
  const existing = RATE_LIMIT_BUCKETS.get(key);

  if (!existing || existing.resetAt <= now) {
    RATE_LIMIT_BUCKETS.set(key, { count: 1, resetAt: now + RATE_LIMIT_WINDOW_MS });
    return null;
  }

  if (existing.count >= RATE_LIMIT_MAX_REQUESTS) {
    const retryAfterSeconds = Math.max(1, Math.ceil((existing.resetAt - now) / 1000));
    const response = jsonError(429, 'Too many requests');
    response.headers.set('Retry-After', String(retryAfterSeconds));
    return response;
  }

  existing.count += 1;
  return null;
}

function buildExpectedOrigin(request: NextRequest): string | null {
  const host = request.headers.get('host');
  if (!host) return null;
  const protocol = request.headers.get('x-forwarded-proto') || 'https';
  return `${protocol}://${host}`;
}

function normalizeOrigin(value: string | null): string | null {
  if (!value) return null;
  try {
    return new URL(value).origin;
  } catch {
    return null;
  }
}

function isAllowedOrigin(request: NextRequest): boolean {
  if (!IS_PRODUCTION) return true;

  const allowed = new Set(ALLOWED_ORIGINS);
  const expectedOrigin = buildExpectedOrigin(request);
  if (expectedOrigin) allowed.add(expectedOrigin);

  const origin = normalizeOrigin(request.headers.get('origin'));
  if (origin) return allowed.has(origin);

  const refererOrigin = normalizeOrigin(request.headers.get('referer'));
  if (refererOrigin) return allowed.has(refererOrigin);

  return false;
}

function validateBodySize(request: NextRequest): NextResponse | null {
  const contentLength = Number(request.headers.get('content-length') || 0);
  if (Number.isFinite(contentLength) && contentLength > MAX_BODY_BYTES) {
    return jsonError(413, 'Request body too large');
  }
  return null;
}

function requireMutatingRequestGuards(request: NextRequest): NextResponse | null {
  const rateLimitResult = checkRateLimit(request);
  if (rateLimitResult) return rateLimitResult;

  if (!isAllowedOrigin(request)) {
    return jsonError(403, 'Forbidden origin');
  }

  return validateBodySize(request);
}

type LoginFlow = 'solana' | 'siwe';
const DEFAULT_LOGIN_FLOW: LoginFlow =
  process.env.NEXT_PUBLIC_CHAIN_MODE === 'base' ? 'siwe' : 'solana';

function parseLoginRequestBody(
  bodyText: string
): { wallet: string; signature: string; message: string; flow: LoginFlow } | null {
  try {
    const parsed = JSON.parse(bodyText) as {
      wallet?: unknown;
      signature?: unknown;
      message?: unknown;
      flow?: unknown;
    };

    if (typeof parsed.wallet !== 'string' || typeof parsed.signature !== 'string' || typeof parsed.message !== 'string') {
      return null;
    }

    if (
      parsed.wallet.length > 96 ||
      parsed.signature.length > 1_024 ||
      parsed.message.length > 4_096
    ) {
      return null;
    }

    const flow: LoginFlow =
      parsed.flow === 'siwe' || parsed.flow === 'solana'
        ? parsed.flow
        : DEFAULT_LOGIN_FLOW;

    return {
      wallet: parsed.wallet.trim(),
      signature: parsed.signature.trim(),
      message: parsed.message,
      flow,
    };
  } catch {
    return null;
  }
}

function getRefreshToken(cookieStore: Awaited<ReturnType<typeof cookies>>): string | undefined {
  const current = cookieStore.get(REFRESH_TOKEN_COOKIE)?.value;
  if (current) return current;

  for (const key of COMPAT_REFRESH_TOKEN_COOKIES) {
    const compat = cookieStore.get(key)?.value;
    if (compat) return compat;
  }

  return undefined;
}

function setRefreshToken(cookieStore: Awaited<ReturnType<typeof cookies>>, refreshToken: string) {
  cookieStore.set(REFRESH_TOKEN_COOKIE, refreshToken, {
    httpOnly: true,
    secure: IS_PRODUCTION,
    sameSite: 'strict',
    maxAge: COOKIE_MAX_AGE,
    path: '/',
  });
  for (const key of COMPAT_REFRESH_TOKEN_COOKIES) {
    cookieStore.delete(key);
  }
}

function clearRefreshToken(cookieStore: Awaited<ReturnType<typeof cookies>>) {
  cookieStore.delete(REFRESH_TOKEN_COOKIE);
  for (const key of COMPAT_REFRESH_TOKEN_COOKIES) {
    cookieStore.delete(key);
  }
}

export async function POST(request: NextRequest) {
  try {
    const guardError = requireMutatingRequestGuards(request);
    if (guardError) return guardError;

    const bodyText = await request.text();
    if (Buffer.byteLength(bodyText, 'utf8') > MAX_BODY_BYTES) {
      return jsonError(413, 'Request body too large');
    }

    const body = parseLoginRequestBody(bodyText);
    if (!body) {
      return jsonError(400, 'Invalid request body');
    }

    const { wallet, signature, message, flow } = body;

    if (!wallet || !signature || !message) {
      return jsonError(400, 'Missing required fields');
    }

    const target = flow === 'siwe' ? `${API_BASE}/auth/siwe/login` : `${API_BASE}/auth/login`;
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };
    if (flow === 'solana') {
      headers.Authorization = `Bearer ${wallet}:${signature}:${message}`;
    }

    const res = await fetch(target, {
      method: 'POST',
      headers,
      body: JSON.stringify({ wallet, signature, message }),
    });

    if (!res.ok) {
      const text = await res.text();
      return jsonError(res.status, text || 'Authentication failed');
    }

    const tokens = await res.json();

    const cookieStore = await cookies();
    setRefreshToken(cookieStore, tokens.refreshToken);

    return NextResponse.json({
      accessToken: tokens.accessToken,
      expiresAt: tokens.expiresAt,
    });
  } catch (error) {
    console.error('Login error:', error);
    return jsonError(500, 'Internal server error');
  }
}

export async function PUT(request: NextRequest) {
  try {
    const guardError = requireMutatingRequestGuards(request);
    if (guardError) return guardError;

    const cookieStore = await cookies();
    const refreshToken = getRefreshToken(cookieStore);

    if (!refreshToken) {
      return jsonError(401, 'No refresh token');
    }

    const res = await fetch(`${API_BASE}/auth/refresh`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${refreshToken}`,
      },
    });

    if (!res.ok) {
      clearRefreshToken(cookieStore);
      return jsonError(res.status, 'Token refresh failed');
    }

    const tokens = await res.json();

    setRefreshToken(cookieStore, tokens.refreshToken);

    return NextResponse.json({
      accessToken: tokens.accessToken,
      expiresAt: tokens.expiresAt,
    });
  } catch (error) {
    console.error('Refresh error:', error);
    return jsonError(500, 'Internal server error');
  }
}

export async function DELETE(request: NextRequest) {
  try {
    const guardError = requireMutatingRequestGuards(request);
    if (guardError) return guardError;

    const cookieStore = await cookies();
    const refreshToken = getRefreshToken(cookieStore);

    if (refreshToken) {
      await fetch(`${API_BASE}/auth/logout`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${refreshToken}`,
        },
      }).catch(() => {
      });
    }

    clearRefreshToken(cookieStore);

    return NextResponse.json({ success: true });
  } catch (error) {
    console.error('Logout error:', error);
    const cookieStore = await cookies();
    clearRefreshToken(cookieStore);
    return NextResponse.json({ success: true });
  }
}

export async function GET() {
  const cookieStore = await cookies();
  const hasRefreshToken = !!getRefreshToken(cookieStore);

  return NextResponse.json({ hasRefreshToken });
}
