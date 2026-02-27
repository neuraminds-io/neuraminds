import { useQuery } from '@tanstack/react-query';
import { api } from '@/lib/api';
import type { MarketFilters, Outcome, PaginatedResponse, Market } from '@/types';

export function useMarkets(filters?: MarketFilters) {
  return useQuery({
    queryKey: ['markets', filters, 'base-api'],
    queryFn: async (): Promise<PaginatedResponse<Market>> => {
      const response = await api.getBaseMarkets({
        limit: filters?.limit || 50,
        offset: filters?.offset || 0,
      });

      let data = [...response.data];
      const requestedCategory = filters?.category?.toLowerCase();
      if (requestedCategory) {
        data = data.filter(
          (market) => market.category.toLowerCase() === requestedCategory
        );
      }

      if (filters?.sort === 'volume') {
        data.sort((a, b) => b.volume24h - a.volume24h);
      } else if (filters?.sort === 'ending') {
        data.sort((a, b) => new Date(a.tradingEnd).getTime() - new Date(b.tradingEnd).getTime());
      }

      const filteredTotal = data.length;
      const offset = filters?.offset || 0;
      const limit = filters?.limit || 50;
      const paged = data.slice(offset, offset + limit);

      return {
        ...response,
        data: paged,
        total: filteredTotal,
        limit,
        offset,
        hasMore: offset + limit < filteredTotal,
      };
    },
    retry: 1,
  });
}

export function useMarket(id: string) {
  return useQuery({
    queryKey: ['market', id, 'base-api'],
    queryFn: async () => api.getBaseMarket(id),
    enabled: !!id,
    retry: 1,
    staleTime: 30000,
  });
}

export function useOrderBook(marketId: string, outcome: Outcome) {
  return useQuery({
    queryKey: ['orderbook', marketId, outcome, 'base-api'],
    queryFn: async () => api.getBaseOrderBook(marketId, outcome),
    enabled: !!marketId,
    refetchInterval: 5000,
  });
}

export function useTrades(
  marketId: string,
  params?: { outcome?: Outcome; limit?: number }
) {
  return useQuery({
    queryKey: ['trades', marketId, params, 'base-api'],
    queryFn: async () => api.getBaseTrades(marketId, params),
    enabled: !!marketId,
  });
}
