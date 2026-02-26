import { NextRequest, NextResponse } from 'next/server';
import { readBaseOrderbook, toApiErrorPayload } from '@/lib/server/baseReadApi';

export async function GET(
  request: NextRequest,
  context: { params: { marketId: string } }
) {
  try {
    return NextResponse.json(
      await readBaseOrderbook(context.params.marketId, request.nextUrl.searchParams)
    );
  } catch (error) {
    const mapped = toApiErrorPayload(error);
    return NextResponse.json(mapped.payload, { status: mapped.status });
  }
}
