"use client";

import { Header, BottomNav } from "@/components/layout";
import { MarketRow } from "@/components/market";
import { SignalChart } from "@/components/market/FeaturedBanner";
import { useMarkets } from "@/hooks";
import type { Market, PaginatedResponse } from "@/types";

const TICKER_TEXT =
  "NEURAMINDS PROTOCOL LIVE ___ AGENT 004 PREDICTING ___ BTC $92,000 [72%] ___ AGI 2026 [14%] ___ MARS LANDING 2029 [61%] ___ SUPERCONDUCTOR LK-99 [08%] ___ NETWORK LATENCY 4MS ___ SETTLEMENT: BASE L2 ___ ";

interface HomePageClientProps {
  initialMarkets?: PaginatedResponse<Market> | null;
}

export default function HomePageClient({ initialMarkets }: HomePageClientProps) {
  const { data: marketsData, isLoading } = useMarkets(
    {
      sort: "volume",
      limit: 12,
    },
    {
      initialData: initialMarkets || undefined,
    }
  );

  const markets = marketsData?.data || [];

  return (
    <div className="min-h-screen bg-bg-base relative overflow-x-hidden">
      <Header />

      <div
        className="absolute top-[-2rem] right-[-4rem] hero-glyph overflow-hidden leading-none"
        aria-hidden
      >
        <span className="block">N</span>
        <span className="block">M</span>
      </div>

      <div className="corner-chrome top-20 left-5 hidden lg:block">
        <div className="w-8 h-8 border border-border flex items-center justify-center text-text-muted text-lg">
          &times;
        </div>
      </div>

      <div className="corner-chrome top-32 left-5 hidden lg:block text-text-muted text-lg tracking-widest">
        &gt;&gt;&gt;
      </div>

      <div className="corner-chrome bottom-16 left-5 hidden lg:block">
        <div className="w-8 h-8 border border-border flex items-center justify-center text-text-muted text-lg">
          +
        </div>
      </div>

      <div
        className="fixed right-5 top-1/2 -translate-y-1/2 writing-mode-vertical text-[11px] uppercase tracking-[0.3em] text-text-muted font-mono hidden lg:block"
        aria-hidden
      >
        SYSTEM_STATUS_OK
      </div>

      <main className="relative max-w-[1200px] mx-auto px-6 sm:px-8 lg:pl-16 lg:pr-16 mb-12">
        <section className="pt-8 pb-8 border-b border-border">
          <p className="text-[11px] uppercase tracking-[0.2em] text-text-secondary font-mono mb-3">
            Predictive Agents // Node 04
          </p>

          <h1 className="text-[clamp(3rem,9vw,7.5rem)] font-bold uppercase leading-[0.88] tracking-[-0.02em] text-text-primary relative">
            NEURAMINDS
            <span
              className="absolute left-0 right-0 h-[3px] bg-accent"
              style={{ top: "52%" }}
              aria-hidden
            />
            <span
              className="absolute left-0 right-0 h-[2px] bg-accent/60"
              style={{ top: "58%" }}
              aria-hidden
            />
          </h1>

          <div className="flex items-center justify-between mt-5 flex-wrap gap-3">
            <div className="flex items-center gap-3">
              <span className="text-[11px] uppercase tracking-[0.18em] text-text-secondary font-mono">
                AGENTIC
              </span>
              <span className="bg-accent text-white px-2.5 py-1 text-[11px] font-bold uppercase tracking-[0.12em]">
                ACTIVE
              </span>
            </div>
            <span className="text-[11px] uppercase tracking-[0.18em] text-text-secondary font-mono">
              Global Volatility:{" "}
              <span className="text-accent font-bold">HIGH</span>
            </span>
          </div>
        </section>

        <section className="py-8 border-b border-border">
          <SignalChart label="ORACLE_A" latency="4MS" />
        </section>

        <section className="py-8 pb-16">
          {isLoading ? (
            <div className="space-y-0">
              {Array.from({ length: 6 }).map((_, i) => (
                <div
                  key={i}
                  className="flex items-center gap-6 py-5 border-b border-border animate-pulse"
                >
                  <div className="w-12 h-4 bg-bg-secondary hidden sm:block" />
                  <div className="flex-1 h-5 bg-bg-secondary" />
                  <div className="w-16 h-4 bg-bg-secondary hidden md:block" />
                  <div className="w-20 h-4 bg-bg-secondary hidden md:block" />
                  <div className="w-16 h-8 bg-bg-secondary" />
                </div>
              ))}
            </div>
          ) : markets.length > 0 ? (
            markets.map((market, i) => (
              <MarketRow key={market.id} market={market} index={i} />
            ))
          ) : (
            <div className="py-12 text-center text-text-muted text-sm uppercase tracking-[0.16em]">
              No active markets
            </div>
          )}
        </section>
      </main>

      <div className="fixed bottom-0 left-0 right-0 bg-bg-primary/95 border-t border-border overflow-hidden z-30 md:z-40">
        <div className="py-2.5 whitespace-nowrap overflow-hidden">
          <span className="animate-marquee text-[11px] text-accent uppercase tracking-[0.16em] font-mono">
            {TICKER_TEXT}
            {TICKER_TEXT}
          </span>
        </div>
      </div>

      <BottomNav />
    </div>
  );
}
