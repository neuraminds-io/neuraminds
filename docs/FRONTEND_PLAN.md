# Frontend Architecture Plan

## Progress Log

### Sprint 1: Foundation (Complete)
- [x] Types & API client (`types/index.ts`, `lib/api.ts`)
- [x] CSS token system (`styles/tokens.css`)
- [x] Tailwind config with CSS variable references
- [x] Base UI components (Button, Input, Card, Badge, Spinner, Select, Tabs)

### Sprint 2: Data Layer (Complete)
- [x] React Query hooks (`useMarkets`, `useOrders`, `usePositions`)
- [x] Auth hook with wallet signature flow
- [x] API client with JWT token management

### Sprint 3: Components (Complete)
- [x] Layout components (Header, BottomNav, PageShell)
- [x] Market components (MarketCard, MarketHeader, MarketStats, PriceBar, MarketList)
- [x] Order components (OrderForm, OrderBook, OrderList)
- [x] Position components (PositionCard, PositionList)

### Sprint 4: Pages (Complete)
- [x] Home page with featured markets and stats
- [x] Markets listing with category filters and sorting
- [x] Market detail with trading UI and order book
- [x] Portfolio with positions and open orders
- [x] Settings page

### Sprint 5: Polish (Complete)
- [x] WebSocket hook for real-time order book updates
- [x] Toast notification system with auto-dismiss
- [x] Error boundary component
- [x] Loading skeleton components (MarketCard, OrderBook, Position, Stats)
- [x] Form validation in OrderForm
- [x] Mock data layer for development without backend (`lib/mockData.ts`)
- [x] Mock-aware hooks (useMarkets, useOrders, usePositions)

### Sprint 6: Testing (Pending)
- [ ] Unit tests for hooks
- [ ] Component tests
- [ ] E2E tests with Playwright

### Sprint 7: Optimization (Pending)
- [ ] Image optimization
- [ ] Bundle analysis
- [ ] Performance monitoring

---

## Design Principles

1. **Minimal styling** - Use semantic CSS classes, avoid hardcoded colors
2. **Token-based theming** - All colors via CSS custom properties
3. **Component isolation** - No style dependencies between components
4. **Type safety** - Shared types between API and UI
5. **Data layer separation** - Hooks for data, components for presentation

## Directory Structure

```
web/src/
├── app/                    # Next.js pages (routing only)
├── components/
│   ├── ui/                 # Base components (Button, Input, Card, etc.)
│   ├── market/             # Market-specific components
│   ├── order/              # Order/trading components
│   ├── position/           # Position components
│   └── layout/             # Header, Nav, Layout shells
├── hooks/
│   ├── useMarkets.ts       # Market data fetching
│   ├── useOrders.ts        # Order management
│   ├── usePositions.ts     # Position tracking
│   ├── useAuth.ts          # Authentication
│   └── useWebSocket.ts     # Real-time updates
├── lib/
│   ├── api.ts              # API client
│   ├── constants.ts        # App constants
│   ├── mockData.ts         # Mock data for dev mode
│   └── utils.ts            # Formatting utilities
├── types/
│   └── index.ts            # Shared TypeScript types
└── styles/
    └── tokens.css          # CSS custom properties
```

## CSS Token System

All colors defined as CSS custom properties in `tokens.css`:

```css
:root {
  /* Semantic tokens - restyle by changing these */
  --color-bg-primary: var(--gray-950);
  --color-bg-secondary: var(--gray-900);
  --color-bg-tertiary: var(--gray-800);
  --color-border: var(--gray-700);
  --color-text-primary: var(--gray-50);
  --color-text-secondary: var(--gray-400);
  --color-accent: var(--blue-500);
  --color-success: var(--green-500);
  --color-danger: var(--red-500);

  /* Spacing scale */
  --space-1: 0.25rem;
  --space-2: 0.5rem;
  --space-3: 0.75rem;
  --space-4: 1rem;
  --space-6: 1.5rem;
  --space-8: 2rem;

  /* Radius */
  --radius-sm: 0.375rem;
  --radius-md: 0.5rem;
  --radius-lg: 0.75rem;
  --radius-full: 9999px;
}
```

## Component Library

### Base UI Components

| Component | Props | Purpose |
|-----------|-------|---------|
| Button | variant, size, disabled, loading | Actions |
| Input | type, label, error, placeholder | Form input |
| Card | children, className | Container |
| Badge | variant, children | Status/category labels |
| Spinner | size | Loading state |
| Tabs | items, active, onChange | Tab navigation |
| Select | options, value, onChange | Dropdown |

### Market Components

| Component | Purpose |
|-----------|---------|
| MarketCard | Market preview in lists |
| MarketHeader | Title, category, status |
| MarketStats | Volume, liquidity, end date |
| PriceBar | Yes/No price visualization |
| MarketList | Paginated market list with loading |

### Order Components

| Component | Purpose |
|-----------|---------|
| OrderForm | Buy/sell interface with outcome tabs |
| OrderBookDisplay | Bids/asks with depth visualization |
| OrderList | User's open/filled orders with cancel |

### Position Components

| Component | Purpose |
|-----------|---------|
| PositionCard | Position summary with P&L |
| PositionList | All user positions |

## Pages

| Route | Component | Data |
|-------|-----------|------|
| `/` | HomePage | Featured markets, stats |
| `/markets` | MarketsPage | Filtered market list |
| `/markets/[id]` | MarketPage | Market detail, order book, trading |
| `/portfolio` | PortfolioPage | User positions, orders |
| `/settings` | SettingsPage | Wallet, preferences |

## Restyling Guide

To restyle the entire app:

1. Edit `styles/tokens.css` - change color values
2. All components use semantic tokens (`--color-bg-primary`, etc.)
3. No hardcoded colors in component files
4. Tailwind config references CSS custom properties

Example theme change:
```css
/* Dark blue theme */
:root {
  --color-bg-primary: #0a1929;
  --color-bg-secondary: #0d2137;
  --color-accent: #5090d3;
}

/* Light theme */
:root.light {
  --color-bg-primary: #ffffff;
  --color-bg-secondary: #f5f5f5;
  --color-text-primary: #1a1a1a;
}
```
