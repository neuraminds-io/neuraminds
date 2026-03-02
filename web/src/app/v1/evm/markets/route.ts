import { NextRequest, NextResponse } from 'next/server';
import { readBaseMarkets, toApiErrorPayload } from '@/lib/server/baseReadApi';

export async function GET(request: NextRequest) {
  try {
    return NextResponse.json(await readBaseMarkets(request.nextUrl.searchParams));
  } catch (error) {
    const mapped = toApiErrorPayload(error);
    return NextResponse.json(mapped.payload, { status: mapped.status });
  }
}
