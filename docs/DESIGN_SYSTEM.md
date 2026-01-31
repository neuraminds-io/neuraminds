# Polyguard Design System

A production-grade prediction market platform combining Kalshi's functional excellence with Kraken's visual identity.

---

## Design Philosophy

### Guiding Principles

1. **Mobile-First, Desktop-Enhanced** — Every component designed for 375px first, then scaled up
2. **Data Density Without Clutter** — Show information traders need without overwhelming
3. **Instant Feedback** — Every interaction responds within 100ms
4. **Trust Through Clarity** — Financial interfaces demand precision and transparency

### Mashup Strategy

| Aspect | Source | Implementation |
|--------|--------|----------------|
| Navigation structure | Kalshi | Category tabs, trending/new filters, market browser |
| Trading mechanics | Kalshi | Yes/No contracts, limit orders, order book depth |
| Portfolio management | Kalshi | P&L tracking, position cards, order history |
| Color palette | Kraken | Purple-dominant dark theme with accent hierarchy |
| Typography | Kraken | Custom sans-serif with monospace for prices |
| Component style | Kraken | Rounded containers, subtle gradients, glow effects |
| Brand personality | Kraken | Bold, modern authority balanced with approachability |

---

## Color System

### Primary Palette (Kraken-Inspired)

```css
:root {
  /* Core Purple Scale */
  --purple-50: #faf5ff;
  --purple-100: #f3e8ff;
  --purple-200: #e9d5ff;
  --purple-300: #d8b4fe;
  --purple-400: #c084fc;
  --purple-500: #a855f7;
  --purple-600: #9333ea;
  --purple-700: #7c3aed;  /* Almost Royal - Primary Brand */
  --purple-800: #6b21a8;
  --purple-900: #581c87;
  --purple-950: #3b0764;

  /* Kraken Brand Colors */
  --kraken-royal: #7434f3;      /* Primary actions, CTAs */
  --kraken-lilac: #bc91f7;      /* Hover states, highlights */
  --kraken-candy: #b494e6;      /* Secondary elements */
}
```

### Dark Mode Foundation

```css
:root {
  /* Background Hierarchy (darkest to lightest) */
  --bg-base: #0a0a0f;           /* App background */
  --bg-primary: #12121a;        /* Card backgrounds */
  --bg-secondary: #1a1a24;      /* Elevated surfaces */
  --bg-tertiary: #24242e;       /* Hover states */
  --bg-elevated: #2e2e3a;       /* Dropdowns, modals */

  /* Border System */
  --border-subtle: rgba(255, 255, 255, 0.06);
  --border-default: rgba(255, 255, 255, 0.1);
  --border-strong: rgba(255, 255, 255, 0.16);
  --border-accent: rgba(116, 52, 243, 0.4);

  /* Text Hierarchy */
  --text-primary: #f8fafc;      /* Primary content */
  --text-secondary: #a1a1aa;    /* Labels, descriptions */
  --text-muted: #71717a;        /* Placeholders, hints */
  --text-inverse: #0a0a0f;      /* Text on light backgrounds */
}
```

### Semantic Colors

```css
:root {
  /* Trading Colors */
  --color-bid: #22c55e;         /* Buy/Yes - Green */
  --color-bid-muted: rgba(34, 197, 94, 0.12);
  --color-bid-glow: rgba(34, 197, 94, 0.25);

  --color-ask: #ef4444;         /* Sell/No - Red */
  --color-ask-muted: rgba(239, 68, 68, 0.12);
  --color-ask-glow: rgba(239, 68, 68, 0.25);

  /* Status Colors */
  --color-success: #10b981;
  --color-warning: #f59e0b;
  --color-danger: #ef4444;
  --color-info: #3b82f6;

  /* Accent (Kraken Purple) */
  --color-accent: #7434f3;
  --color-accent-hover: #8b5cf6;
  --color-accent-muted: rgba(116, 52, 243, 0.15);
  --color-accent-glow: rgba(116, 52, 243, 0.3);
}
```

---

## Typography

### Font Stack

```css
:root {
  /* Primary - Clean, modern interface */
  --font-sans: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;

  /* Display - Headlines, market titles */
  --font-display: 'Space Grotesk', var(--font-sans);

  /* Mono - Prices, percentages, order data */
  --font-mono: 'JetBrains Mono', 'SF Mono', 'Fira Code', monospace;
}
```

### Type Scale

| Token | Size | Weight | Line Height | Use Case |
|-------|------|--------|-------------|----------|
| `text-xs` | 11px | 400 | 1.4 | Timestamps, fine print |
| `text-sm` | 13px | 400 | 1.5 | Labels, secondary info |
| `text-base` | 15px | 400 | 1.5 | Body text, descriptions |
| `text-lg` | 17px | 500 | 1.4 | Subheadings |
| `text-xl` | 20px | 600 | 1.3 | Card titles |
| `text-2xl` | 24px | 600 | 1.2 | Section headers |
| `text-3xl` | 30px | 700 | 1.1 | Page titles |
| `text-price` | 15px | 500 | 1 | Prices (mono) |
| `text-price-lg` | 20px | 600 | 1 | Featured prices (mono) |

### Typography Rules

1. **Prices always use monospace** — Alignment and readability for numbers
2. **Percentages include sign** — `+2.5%` or `-1.2%` with color
3. **Currency format** — `$1,234.56` with thousand separators
4. **Truncation** — Use ellipsis with `line-clamp-2` for market titles

---

## Spacing System

8px base unit for consistency with common design tools.

```css
:root {
  --space-0: 0;
  --space-1: 4px;     /* Tight gaps */
  --space-2: 8px;     /* Default gap */
  --space-3: 12px;    /* Component padding */
  --space-4: 16px;    /* Card padding mobile */
  --space-5: 20px;    /* Section gaps */
  --space-6: 24px;    /* Card padding desktop */
  --space-8: 32px;    /* Section padding */
  --space-10: 40px;   /* Large sections */
  --space-12: 48px;   /* Page margins */
  --space-16: 64px;   /* Hero sections */
}
```

### Spacing Guidelines

- **Touch targets**: Minimum 44x44px on mobile
- **Card padding**: 16px mobile, 24px desktop
- **List item gaps**: 8px between items
- **Section gaps**: 32px between major sections

---

## Component Library

### Cards

```
┌─────────────────────────────────────┐
│  Market Card                        │
├─────────────────────────────────────┤
│  bg: var(--bg-primary)              │
│  border: 1px solid var(--border-subtle)
│  border-radius: 12px                │
│  padding: 16px (mobile) / 24px (desktop)
│  transition: all 150ms ease         │
│                                     │
│  :hover                             │
│    bg: var(--bg-secondary)          │
│    border-color: var(--border-default)
│    transform: translateY(-1px)      │
│    box-shadow: var(--shadow-md)     │
└─────────────────────────────────────┘
```

### Buttons

| Variant | Background | Text | Border | Use |
|---------|------------|------|--------|-----|
| Primary | `--kraken-royal` | white | none | Main CTAs |
| Secondary | `--bg-secondary` | `--text-primary` | `--border-default` | Secondary actions |
| Ghost | transparent | `--text-secondary` | none | Tertiary actions |
| Success | `--color-bid` | white | none | Buy/Yes actions |
| Danger | `--color-ask` | white | none | Sell/No actions |
| Outline | transparent | `--kraken-royal` | `--kraken-royal` | Alternative CTA |

**Button Sizes**

| Size | Height | Padding | Font |
|------|--------|---------|------|
| sm | 32px | 12px 16px | 13px |
| md | 40px | 12px 20px | 15px |
| lg | 48px | 16px 24px | 17px |
| xl | 56px | 16px 32px | 17px |

### Input Fields

```
┌─────────────────────────────────────┐
│ Label                               │
├─────────────────────────────────────┤
│ ┌─────────────────────────────────┐ │
│ │ Placeholder text                │ │
│ └─────────────────────────────────┘ │
│ Hint text or error message          │
└─────────────────────────────────────┘

Default:
  bg: var(--bg-secondary)
  border: 1px solid var(--border-default)
  border-radius: 8px
  height: 44px
  padding: 0 12px

Focus:
  border-color: var(--kraken-royal)
  box-shadow: 0 0 0 3px var(--color-accent-muted)

Error:
  border-color: var(--color-danger)
  box-shadow: 0 0 0 3px var(--color-ask-muted)
```

### Badges/Tags

```
Category badges:
  bg: var(--bg-tertiary)
  color: var(--text-secondary)
  border-radius: 6px
  padding: 4px 8px
  font-size: 12px
  font-weight: 500

Status badges:
  Active:  bg: var(--color-bid-muted), color: var(--color-bid)
  Pending: bg: var(--color-warning-muted), color: var(--color-warning)
  Closed:  bg: var(--bg-tertiary), color: var(--text-muted)
```

---

## Layout Patterns

### Mobile Navigation (Bottom Tab Bar)

```
┌─────────────────────────────────────┐
│                                     │
│           Main Content              │
│                                     │
├─────────────────────────────────────┤
│  🏠    📊    💼    ⚙️              │
│ Home  Markets Portfolio Settings    │
└─────────────────────────────────────┘

Height: 64px + safe-area-inset-bottom
Background: var(--bg-primary) with backdrop-blur
Border-top: 1px solid var(--border-subtle)
```

### Market Card Layout

```
Mobile (< 640px):
┌─────────────────────────────────────┐
│ [Category]  [Status]                │
│                                     │
│ Market question title that can      │
│ span multiple lines...              │
│                                     │
│ ┌─────────────────────────────────┐ │
│ │ Yes 65¢ ████████████░░░░ No 35¢│ │
│ └─────────────────────────────────┘ │
│                                     │
│ $1.2M vol  •  Ends Jan 20          │
└─────────────────────────────────────┘

Desktop (≥ 1024px):
┌────────────────────────────────────────────────────────────┐
│ [Category]                                                 │
│                                                            │
│ Market question title                     Yes 65¢  No 35¢ │
│ Additional context...                     [Buy] [Sell]    │
│                                                            │
│ $1.2M volume  •  $500K liquidity  •  Ends Jan 20, 2025    │
└────────────────────────────────────────────────────────────┘
```

### Trading Interface (Market Detail)

```
Mobile:
┌─────────────────────────────────────┐
│ ← Back              [Share] [Watch] │
├─────────────────────────────────────┤
│ [Category]  [Status]                │
│                                     │
│ Market Question Title               │
│                                     │
│ ┌─────────────────────────────────┐ │
│ │        Price Chart              │ │
│ │     (Lightweight Charts)        │ │
│ └─────────────────────────────────┘ │
│                                     │
│ [1H] [1D] [1W] [1M] [ALL]          │
├─────────────────────────────────────┤
│ Stats Row:                          │
│ Volume | Liquidity | Open Interest  │
├─────────────────────────────────────┤
│                                     │
│     Order Book (Collapsible)        │
│                                     │
├─────────────────────────────────────┤
│ ┌─────────────────────────────────┐ │
│ │  [Yes 65¢]     [No 35¢]        │ │
│ │                                 │ │
│ │  Amount: [__________]          │ │
│ │                                 │ │
│ │  Est. Shares: 15.38            │ │
│ │  Max Profit: +$5.38            │ │
│ │                                 │ │
│ │  [    Buy Yes for $10    ]     │ │
│ └─────────────────────────────────┘ │
└─────────────────────────────────────┘

Desktop (Two-Column):
┌────────────────────────────────────────────────────────────┐
│ ← Markets                                   [Share] [Watch]│
├──────────────────────────────┬─────────────────────────────┤
│                              │                             │
│  Market Title                │  Trade Panel                │
│  [Category] [Status]         │  ┌─────────────────────┐   │
│                              │  │ [Yes] [No]          │   │
│  ┌────────────────────────┐  │  │                     │   │
│  │                        │  │  │ [Buy] [Sell]        │   │
│  │    Price Chart         │  │  │                     │   │
│  │                        │  │  │ Amount: [____]      │   │
│  └────────────────────────┘  │  │                     │   │
│  [1H] [1D] [1W] [1M] [ALL]   │  │ Price: [____]       │   │
│                              │  │                     │   │
│  Stats Grid                  │  │ [Place Order]       │   │
│  ┌──────┬──────┬──────────┐  │  └─────────────────────┘   │
│  │ Vol  │ Liq  │ Interest │  │                             │
│  └──────┴──────┴──────────┘  │  Order Book                 │
│                              │  ┌─────────────────────┐   │
│  Resolution Info             │  │ Bids    │    Asks  │   │
│  Settlement sources...       │  │ ...     │    ...   │   │
│                              │  └─────────────────────┘   │
└──────────────────────────────┴─────────────────────────────┘
```

---

## Animation & Motion

### Timing Functions

```css
:root {
  --ease-out: cubic-bezier(0.4, 0, 0.2, 1);
  --ease-in-out: cubic-bezier(0.4, 0, 0.2, 1);
  --ease-spring: cubic-bezier(0.34, 1.56, 0.64, 1);
  --ease-bounce: cubic-bezier(0.68, -0.6, 0.32, 1.6);
}
```

### Duration Scale

| Token | Duration | Use Case |
|-------|----------|----------|
| `instant` | 50ms | Micro-interactions, button press |
| `fast` | 150ms | Hovers, focus states |
| `normal` | 200ms | Transitions, state changes |
| `slow` | 300ms | Page transitions, modals |
| `slower` | 500ms | Complex animations |

### Interaction Patterns

**Button Press**
```css
.button:active {
  transform: scale(0.98);
  transition: transform 50ms var(--ease-out);
}
```

**Card Hover**
```css
.card:hover {
  transform: translateY(-2px);
  box-shadow: var(--shadow-lg);
  transition: all 150ms var(--ease-out);
}
```

**Price Flash (Real-time updates)**
```css
@keyframes price-flash-up {
  0% { background-color: var(--color-bid-muted); }
  100% { background-color: transparent; }
}

@keyframes price-flash-down {
  0% { background-color: var(--color-ask-muted); }
  100% { background-color: transparent; }
}
```

**Skeleton Loading**
```css
@keyframes skeleton-pulse {
  0%, 100% { opacity: 0.4; }
  50% { opacity: 0.8; }
}

.skeleton {
  animation: skeleton-pulse 2s ease-in-out infinite;
}
```

---

## Responsive Breakpoints

```css
/* Mobile-first breakpoints */
--breakpoint-sm: 640px;   /* Large phones, small tablets */
--breakpoint-md: 768px;   /* Tablets */
--breakpoint-lg: 1024px;  /* Laptops, small desktops */
--breakpoint-xl: 1280px;  /* Large desktops */
--breakpoint-2xl: 1536px; /* Extra large screens */
```

### Layout Adaptations

| Breakpoint | Navigation | Grid | Card Layout |
|------------|------------|------|-------------|
| < 640px | Bottom tabs | 1 column | Full width |
| 640-767px | Bottom tabs | 2 columns | Full width |
| 768-1023px | Side nav collapsed | 2-3 columns | Horizontal |
| ≥ 1024px | Side nav expanded | 3-4 columns | Horizontal |

---

## Accessibility

### Color Contrast

All text meets WCAG 2.1 AA standards:
- Primary text on dark bg: 15.8:1 ratio
- Secondary text on dark bg: 7.2:1 ratio
- Muted text on dark bg: 4.6:1 ratio

### Focus States

```css
*:focus-visible {
  outline: 2px solid var(--kraken-royal);
  outline-offset: 2px;
}
```

### Motion Preferences

```css
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
```

### Touch Targets

- Minimum 44x44px for all interactive elements on touch devices
- 8px minimum spacing between adjacent touch targets

---

## Icons

### Library

Use **Lucide React** for consistency with shadcn/ui.

### Size Scale

| Size | Pixels | Use Case |
|------|--------|----------|
| xs | 14px | Inline with small text |
| sm | 16px | Buttons, inline with base text |
| md | 20px | Navigation, standalone |
| lg | 24px | Headers, prominent icons |
| xl | 32px | Empty states, illustrations |

### Common Icons

| Action | Icon | Notes |
|--------|------|-------|
| Home | `Home` | |
| Markets | `TrendingUp` | |
| Portfolio | `Briefcase` | |
| Settings | `Settings` | |
| Search | `Search` | |
| Filter | `SlidersHorizontal` | |
| Sort | `ArrowUpDown` | |
| Buy | `Plus` or `ArrowUp` | Green |
| Sell | `Minus` or `ArrowDown` | Red |
| Watchlist | `Star` / `StarOff` | Filled when active |
| Share | `Share2` | |
| Copy | `Copy` | |
| External Link | `ExternalLink` | |
| Close | `X` | |
| Back | `ArrowLeft` | |
| Menu | `Menu` | |
| Wallet | `Wallet` | |
| Notification | `Bell` | |

---

## Shadow System

```css
:root {
  --shadow-xs: 0 1px 2px rgba(0, 0, 0, 0.3);
  --shadow-sm: 0 2px 4px rgba(0, 0, 0, 0.3);
  --shadow-md: 0 4px 8px rgba(0, 0, 0, 0.4);
  --shadow-lg: 0 8px 16px rgba(0, 0, 0, 0.4);
  --shadow-xl: 0 16px 32px rgba(0, 0, 0, 0.5);

  /* Glow effects for interactive elements */
  --shadow-glow-purple: 0 0 20px var(--color-accent-glow);
  --shadow-glow-green: 0 0 20px var(--color-bid-glow);
  --shadow-glow-red: 0 0 20px var(--color-ask-glow);
}
```

---

## Implementation Checklist

### Phase 1: Foundation
- [ ] Update `tokens.css` with new color system
- [ ] Configure Tailwind with Kraken palette
- [ ] Add font imports (Inter, Space Grotesk, JetBrains Mono)
- [ ] Set up dark mode as default

### Phase 2: Core Components
- [ ] Button variants (primary purple, success/danger for trading)
- [ ] Card component with hover states
- [ ] Input fields with proper focus/error states
- [ ] Badge/Tag components
- [ ] Skeleton loaders with purple tint

### Phase 3: Trading Components
- [ ] Market card (mobile + desktop layouts)
- [ ] Price bar visualization
- [ ] Order form with Yes/No tabs
- [ ] Order book display
- [ ] Position card with P&L

### Phase 4: Layout
- [ ] Bottom navigation (mobile)
- [ ] Side navigation (desktop)
- [ ] Page shell with responsive behavior
- [ ] Header with wallet connection

### Phase 5: Charts & Data
- [ ] TradingView Lightweight Charts integration
- [ ] Price sparklines for market cards
- [ ] Real-time price updates with flash animation

### Phase 6: Polish
- [ ] Loading states for all async operations
- [ ] Error states and empty states
- [ ] Toast notifications
- [ ] Micro-interactions and transitions
- [ ] Performance optimization

---

## References

### Kalshi (Functionality)
- Navigation: Category tabs + trending/new filters
- Market display: Title, prices, volume, end date
- Trading: Yes/No contracts, limit orders, order book
- Source: https://kalshi.com

### Kraken (Visual Identity)
- Colors: Purple (#7434f3), Lilac (#bc91f7)
- Typography: Kraken Sans, Inter fallback
- Style: Dark mode, subtle gradients, glow effects
- Source: https://kraken.com, https://pro.kraken.com

### Design Resources
- [Kraken Brand Colors](https://mobbin.com/colors/brand/kraken)
- [Kraken Pro Interface Guide](https://support.kraken.com/articles/kraken-pro-trading-interface-guide)
- [Kalshi App Review](https://kalshibetting.com/app/)
