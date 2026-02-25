'use client';

import { useState } from 'react';
import { Flame, Clock } from 'lucide-react';
import { Header, BottomNav } from '@/components/layout';
import { MarketList, FeaturedBanner } from '@/components/market';
import { useMarkets } from '@/hooks';
import { cn } from '@/lib/utils';
import { CATEGORIES } from '@/lib/constants';

const TRENDING_TOPICS = [
  'For you',
  'Bitcoin',
  'Elections',
  'Fed Rates',
  'AI',
  'Sports',
  'Crypto',
  'Tech IPOs',
];

type SortTab = 'trending' | 'new';

export default function HomePage() {
  const [category, setCategory] = useState('All');
  const [sortTab, setSortTab] = useState<SortTab>('trending');

  const { data: featuredData, isLoading: featuredLoading } = useMarkets({
    limit: 6,
    sort: 'volume',
  });

  const { data: marketsData, isLoading } = useMarkets({
    category: category === 'All' ? undefined : category.toLowerCase(),
    sort: sortTab === 'trending' ? 'volume' : 'newest',
    limit: 20,
  });

  const featuredMarkets = featuredData?.data || [];
  const markets = marketsData?.data || [];

  return (
    <div className="min-h-screen bg-bg-base">
      <Header />
      {/* Category Filter Bar */}
      <div className="sticky top-14 z-40 bg-bg-primary border-b border-border">
        <div className="max-w-[1400px] mx-auto px-4 sm:px-6">
          <div className="flex items-center gap-4 py-3 overflow-x-auto scrollbar-hide">
            {/* Sort tabs */}
            <div className="flex items-center gap-1 flex-shrink-0">
              <button
                onClick={() => setSortTab('trending')}
                className={cn(
                  'flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-medium transition-colors cursor-pointer',
                  sortTab === 'trending'
                    ? '[&>span]:bg-gradient-to-r [&>span]:from-accent [&>span]:to-[#ff8b5f] [&>span]:bg-clip-text [&>span]:text-transparent [&>svg]:text-accent'
                    : 'text-text-secondary hover:bg-bg-hover'
                )}
              >
                <Flame className="w-3.5 h-3.5" />
                <span>Trending</span>
              </button>
              <button
                onClick={() => setSortTab('new')}
                className={cn(
                  'flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-medium transition-colors cursor-pointer',
                  sortTab === 'new'
                    ? '[&>span]:bg-gradient-to-r [&>span]:from-accent [&>span]:to-[#ff8b5f] [&>span]:bg-clip-text [&>span]:text-transparent [&>svg]:text-accent'
                    : 'text-text-secondary hover:bg-bg-hover'
                )}
              >
                <Clock className="w-3.5 h-3.5" />
                <span>New</span>
              </button>
            </div>

            <div className="w-px h-5 bg-border flex-shrink-0" />

            {/* Category pills */}
            <div className="flex items-center gap-1.5">
              {CATEGORIES.map((cat) => (
                <button
                  key={cat}
                  onClick={() => setCategory(cat)}
                  className={cn(
                    'px-3 py-1.5 rounded-lg text-sm font-medium whitespace-nowrap transition-colors cursor-pointer',
                    category === cat
                      ? 'bg-bg-tertiary text-text-primary'
                      : 'text-text-secondary hover:bg-bg-hover hover:text-text-primary'
                  )}
                >
                  {cat}
                </button>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Topic Pills */}
      <div className="bg-bg-primary border-b border-border">
        <div className="max-w-[1400px] mx-auto px-4 sm:px-6">
          <div className="flex items-center gap-2 py-3 overflow-x-auto scrollbar-hide">
            {TRENDING_TOPICS.map((topic, i) => (
              <button
                key={topic}
                className={cn(
                  'px-4 py-2 rounded-full text-sm font-medium whitespace-nowrap transition-all cursor-pointer',
                  'border',
                  i === 0
                    ? 'bg-transparent text-accent border-accent'
                    : 'bg-bg-primary text-text-secondary border-border hover:border-border-hover hover:text-text-primary'
                )}
              >
                {topic}
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="max-w-[1400px] mx-auto px-4 sm:px-6 py-6">
        {/* Featured Banner - Kalshi style hero card */}
        {category === 'All' && (
          <section className="mb-8">
            <FeaturedBanner markets={featuredMarkets} />
          </section>
        )}

        {/* Market Grid */}
        <section>
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-text-primary">
              {category === 'All' ? 'All Markets' : category}
            </h2>
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
      </div>

      <BottomNav />
    </div>
  );
}
