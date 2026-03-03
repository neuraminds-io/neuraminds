"use client";

import Link from "next/link";
import { useState } from "react";
import {
  Flame,
  Clock,
  ArrowUpRight,
  Sparkles,
  Waves,
  ShieldCheck,
  Zap,
} from "lucide-react";
import { Header, BottomNav } from "@/components/layout";
import { MarketList, FeaturedBanner } from "@/components/market";
import { useMarkets } from "@/hooks";
import { cn } from "@/lib/utils";
import { CATEGORIES } from "@/lib/constants";

const TRENDING_TOPICS = [
  "For you",
  "Bitcoin",
  "Elections",
  "Fed Rates",
  "AI",
  "Sports",
  "Crypto",
  "Tech IPOs",
];

type SortTab = "trending" | "new";

const SIGNAL_CHIPS = [
  { label: "Settlement", value: "Base L2", icon: ShieldCheck },
  { label: "Runtime", value: "Agent mesh", icon: Zap },
  { label: "Signal feed", value: "X / RSS / On-chain", icon: Waves },
];

export default function HomePage() {
  const [category, setCategory] = useState("All");
  const [sortTab, setSortTab] = useState<SortTab>("trending");

  const { data: featuredData, isLoading: featuredLoading } = useMarkets({
    limit: 6,
    sort: "volume",
  });

  const { data: marketsData, isLoading } = useMarkets({
    category: category === "All" ? undefined : category.toLowerCase(),
    sort: sortTab === "trending" ? "volume" : "newest",
    limit: 20,
  });

  const featuredMarkets = featuredData?.data || [];
  const markets = marketsData?.data || [];

  return (
    <div className="min-h-screen bg-bg-base">
      <Header />

      <main className="max-w-[1400px] mx-auto px-4 sm:px-6 py-8 space-y-8">
        {/* Hero */}
        <section className="relative overflow-hidden border border-border/80 bg-bg-primary/95 micro-surface dot-noise">
          <div className="micro-stripes" />
          <div
            className="micrographic-signal"
            style={{ right: -60, top: -40 }}
            aria-hidden
          />
          <div
            className="micrographic-grid"
            style={{ right: 20, bottom: 8, opacity: 0.12 }}
            aria-hidden
          />
          <div className="tape-stripe" aria-hidden />

          <div className="relative grid gap-10 lg:grid-cols-[1.05fr,0.95fr] p-6 sm:p-8">
            <div className="space-y-6">
              <div className="flex flex-wrap items-center gap-4 text-xs uppercase tracking-[0.2em] text-text-secondary">
                <span>Space Tested</span>
                <span>/</span>
                <span>Machine-Native</span>
                <span>/</span>
                <span className="text-accent">Energy Capture</span>
              </div>

              <div className="space-y-3">
                <p className="label-slab text-text-secondary">
                  Neuraminds Orbital Systems / Base L2
                </p>
                <h1 className="text-3xl sm:text-4xl font-bold hero-serif leading-tight text-text-primary">
                  POWER GRID — AUTONOMOUS AGENT LATTICE (OSP-01) © 2026
                </h1>
                <p className="text-sm sm:text-base text-text-secondary max-w-2xl">
                  Continuous execution, closed-loop regulation, and intent-level
                  audits delivered by a dual-chain runtime. The grid never
                  sleeps.
                </p>
              </div>

              <div className="flex flex-wrap gap-3">
                <Link
                  href="/agents"
                  className="inline-flex items-center gap-2 h-11 px-5 text-sm font-semibold text-white bg-accent hover:bg-accent-hover hover:shadow-md hover:shadow-accent/25 transition-transform duration-fast hover:-translate-y-0.5"
                >
                  Deploy agent
                  <ArrowUpRight className="w-4 h-4" />
                </Link>
                <Link
                  href="/docs/api"
                  className="inline-flex items-center gap-2 h-11 px-4 text-sm font-medium border border-border text-text-primary bg-bg-secondary hover:bg-bg-hover transition-colors"
                >
                  API surface
                </Link>
                <div className="hidden sm:flex items-center gap-2 h-11 px-4 text-xs uppercase tracking-[0.12em] border border-border text-text-secondary bg-bg-tertiary/70">
                  <Sparkles className="w-4 h-4 text-accent" /> Live data mesh
                </div>
              </div>

              <div className="flex flex-wrap gap-2">
                {SIGNAL_CHIPS.map((chip) => (
                  <div key={chip.label} className="micro-chip">
                    <chip.icon className="w-4 h-4 text-text-muted" />
                    <span className="text-text-muted">{chip.label}</span>
                    <strong>{chip.value}</strong>
                  </div>
                ))}
              </div>

              <div className="flex flex-wrap gap-3">
                {TRENDING_TOPICS.map((topic, i) => (
                  <button
                    key={topic}
                    className={cn(
                      "px-3 py-1.5 text-xs sm:text-sm font-medium border whitespace-nowrap transition-all cursor-pointer",
                      i === 0
                        ? "border-accent text-accent bg-accent/5"
                        : "border-border text-text-secondary hover:border-border-hover hover:text-text-primary",
                    )}
                  >
                    {topic}
                  </button>
                ))}
              </div>
            </div>

            <div className="relative">
              <div
                className="micrographic-grid"
                style={{ right: -30, top: -10, opacity: 0.16 }}
                aria-hidden
              />
              <div className="absolute inset-0 bg-gradient-to-br from-[#d9e7ff]/60 via-transparent to-[#ffead8]/60 blur-3xl" />
              <div className="relative border border-border/60 bg-bg-secondary/70 backdrop-blur-sm p-5 sm:p-6 h-full flex flex-col gap-4 dot-noise">
                <div className="flex items-center justify-between text-xs uppercase tracking-[0.14em] text-text-secondary">
                  <span>Telemetry</span>
                  <span className="text-text-muted">Real-time</span>
                </div>
                <div className="grid grid-cols-2 gap-3 text-sm">
                  <div className="p-4 border border-border bg-bg-primary">
                    <div className="text-text-secondary">Markets live</div>
                    <div className="mt-1 text-2xl font-semibold">
                      {marketsData?.total || markets.length || "—"}
                    </div>
                    <div className="text-xs text-text-muted mt-1">
                      Synced with Base
                    </div>
                  </div>
                  <div className="p-4 border border-border bg-bg-primary">
                    <div className="text-text-secondary">Agent uptime</div>
                    <div className="mt-1 text-2xl font-semibold">99.3%</div>
                    <div className="text-xs text-text-muted mt-1">
                      Rolling 30d
                    </div>
                  </div>
                  <div className="p-4 border border-border bg-bg-primary">
                    <div className="text-text-secondary">24h volume</div>
                    <div className="mt-1 text-2xl font-semibold">
                      ${(featuredMarkets[0]?.totalVolume || 0).toLocaleString()}
                    </div>
                    <div className="text-xs text-text-muted mt-1">
                      Across venues
                    </div>
                  </div>
                  <div className="p-4 border border-border bg-bg-primary">
                    <div className="text-text-secondary">Latency</div>
                    <div className="mt-1 text-2xl font-semibold">180ms</div>
                    <div className="text-xs text-text-muted mt-1">
                      Settlement path
                    </div>
                  </div>
                </div>
                <div className="flex items-center gap-3 text-sm text-text-secondary">
                  <div className="w-10 h-10 rounded-full bg-accent/10 border border-accent/40 flex items-center justify-center">
                    <Flame className="w-5 h-5 text-accent" />
                  </div>
                  <div>
                    <div className="font-semibold text-text-primary">
                      Grid never sleeps
                    </div>
                    <div className="text-xs text-text-muted">
                      Closed-loop execution with replay guards and dual
                      registries.
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </section>

        {/* Filter rail */}
        <section className="border border-border bg-bg-primary/90 micro-surface sticky top-16 z-30">
          <div className="micro-stripes" />
          <div className="relative flex flex-col gap-3 px-4 sm:px-5 py-4">
            <div className="flex items-center justify-between gap-3">
              <div className="flex items-center gap-2 text-xs uppercase tracking-[0.18em] text-text-secondary">
                <span className="w-2 h-2 rounded-full bg-accent" />
                <span>Browse</span>
              </div>
              <div className="flex items-center gap-1">
                <button
                  onClick={() => setSortTab("trending")}
                  className={cn(
                    "flex items-center gap-1.5 px-3 py-1.5 text-xs sm:text-sm font-medium transition-all cursor-pointer",
                    sortTab === "trending"
                      ? "text-text-primary bg-accent/10 border border-accent"
                      : "text-text-secondary border border-border hover:border-border-hover",
                  )}
                >
                  <Flame className="w-3.5 h-3.5" /> Trending
                </button>
                <button
                  onClick={() => setSortTab("new")}
                  className={cn(
                    "flex items-center gap-1.5 px-3 py-1.5 text-xs sm:text-sm font-medium transition-all cursor-pointer",
                    sortTab === "new"
                      ? "text-text-primary bg-accent/10 border border-accent"
                      : "text-text-secondary border border-border hover:border-border-hover",
                  )}
                >
                  <Clock className="w-3.5 h-3.5" /> New
                </button>
              </div>
            </div>

            <div className="flex gap-2 overflow-x-auto scrollbar-hide pb-1">
              {CATEGORIES.map((cat) => (
                <button
                  key={cat}
                  onClick={() => setCategory(cat)}
                  className={cn(
                    "px-3 py-1.5 text-sm font-medium whitespace-nowrap transition-all cursor-pointer border",
                    category === cat
                      ? "border-accent text-text-primary bg-accent/8"
                      : "border-border text-text-secondary hover:border-border-hover hover:text-text-primary",
                  )}
                >
                  {cat}
                </button>
              ))}
            </div>
          </div>
        </section>

        {/* Featured Banner */}
        {category === "All" && (
          <section>
            <FeaturedBanner markets={featuredMarkets} />
          </section>
        )}

        {/* Market Grid */}
        <section className="space-y-3">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-xs uppercase tracking-[0.18em] text-text-secondary">
                Live markets
              </p>
              <h2 className="text-xl font-semibold text-text-primary">
                {category === "All" ? "All Markets" : category}
              </h2>
            </div>
            <span className="text-sm text-text-muted">
              {marketsData?.total || 0} markets
            </span>
          </div>

          <MarketList
            markets={markets}
            isLoading={isLoading || featuredLoading}
            columns={4}
            emptyMessage="No markets found in this category"
          />
        </section>
      </main>

      <BottomNav />
    </div>
  );
}
