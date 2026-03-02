# Polyguard Visual Identity

Privacy-first prediction market terminal. Bloomberg-meets-crypto aesthetic.

---

## Design Philosophy

**Reference:** Poly Terminal, Verso, Panther - institutional trading terminals with data density and professional credibility.

**Core Principle:** Trust through precision. Every pixel communicates competence and security.

**Signature:** The interface should feel like a command center - dense with information but never chaotic. Users are making financial decisions with real stakes. The design must inspire confidence.

---

## Color System

### Foundation (Dark Mode Only)

```css
:root {
  /* Backgrounds - Deep slate, not pure black */
  --bg-base: #0a0f1a;        /* App canvas */
  --bg-surface: #0f1629;     /* Cards, panels */
  --bg-elevated: #151d32;    /* Dropdowns, modals */
  --bg-hover: #1a2540;       /* Interactive hover */

  /* Foreground - High contrast text */
  --fg-primary: #f1f5f9;     /* Primary text */
  --fg-secondary: #94a3b8;   /* Secondary text */
  --fg-muted: #64748b;       /* Tertiary/disabled */
  --fg-faint: #475569;       /* Hints, placeholders */

  /* Borders - Subtle definition */
  --border-default: rgba(148, 163, 184, 0.08);
  --border-subtle: rgba(148, 163, 184, 0.04);
  --border-strong: rgba(148, 163, 184, 0.16);

  /* Brand - Electric blue (trust + tech) */
  --brand-primary: #3b82f6;
  --brand-hover: #2563eb;
  --brand-subtle: rgba(59, 130, 246, 0.12);

  /* Semantic - Trading colors */
  --success: #10b981;        /* Profit, YES, positive */
  --success-subtle: rgba(16, 185, 129, 0.12);
  --danger: #ef4444;         /* Loss, NO, negative */
  --danger-subtle: rgba(239, 68, 68, 0.12);
  --warning: #f59e0b;        /* Caution, pending */
  --warning-subtle: rgba(245, 158, 11, 0.12);

  /* Special - Privacy indicator */
  --private: #8b5cf6;        /* Private mode accent */
  --private-subtle: rgba(139, 92, 246, 0.12);
}
```

### Color Usage Rules

| Element | Color | Rationale |
|---------|-------|-----------|
| App background | `--bg-base` | Deep slate reduces eye strain during long sessions |
| Cards/panels | `--bg-surface` | Subtle elevation, not dramatic |
| Interactive hover | `--bg-hover` | Feedback without distraction |
| Primary text | `--fg-primary` | High contrast for data readability |
| Labels/metadata | `--fg-secondary` | Hierarchy without competition |
| Primary actions | `--brand-primary` | Trust blue, institutional feel |
| YES positions | `--success` | Universal profit/positive |
| NO positions | `--danger` | Universal loss/negative |
| Private mode | `--private` | Distinct security indicator |

---

## Typography

### Font Stack

```css
:root {
  /* Primary UI - Clean, professional, technical */
  --font-sans: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;

  /* Data/Numbers - Monospace for alignment */
  --font-mono: 'JetBrains Mono', 'Fira Code', 'SF Mono', Consolas, monospace;

  /* Display/Headers - Optional accent font */
  --font-display: 'Inter', var(--font-sans);
}
```

**Why Inter + JetBrains Mono:**
- Inter: Designed for screens, excellent at small sizes, professional without being cold
- JetBrains Mono: Ligatures for code, tabular figures for prices, technical credibility
- Avoids crypto-cliche fonts like Orbitron (too sci-fi) while maintaining tech identity

### Type Scale

```css
:root {
  /* Size scale - 4px base */
  --text-xs: 0.75rem;    /* 12px - Metadata, timestamps */
  --text-sm: 0.875rem;   /* 14px - Secondary content */
  --text-base: 1rem;     /* 16px - Body text */
  --text-lg: 1.125rem;   /* 18px - Emphasis */
  --text-xl: 1.25rem;    /* 20px - Section headers */
  --text-2xl: 1.5rem;    /* 24px - Page titles */
  --text-3xl: 1.875rem;  /* 30px - Hero numbers */

  /* Weight scale */
  --font-normal: 400;
  --font-medium: 500;
  --font-semibold: 600;
  --font-bold: 700;

  /* Line heights */
  --leading-tight: 1.25;
  --leading-normal: 1.5;
  --leading-relaxed: 1.625;

  /* Letter spacing */
  --tracking-tight: -0.025em;
  --tracking-normal: 0;
  --tracking-wide: 0.025em;
}
```

### Typography Rules

| Element | Size | Weight | Font | Tracking |
|---------|------|--------|------|----------|
| Page title | `--text-2xl` | 600 | Sans | Tight |
| Section header | `--text-lg` | 600 | Sans | Normal |
| Body text | `--text-base` | 400 | Sans | Normal |
| Labels | `--text-sm` | 500 | Sans | Wide |
| Prices/amounts | `--text-base` | 500 | Mono | Normal |
| Percentages | `--text-sm` | 600 | Mono | Normal |
| Timestamps | `--text-xs` | 400 | Mono | Normal |

---

## Spacing System

**Base unit:** 4px

```css
:root {
  --space-0: 0;
  --space-1: 0.25rem;   /* 4px */
  --space-2: 0.5rem;    /* 8px */
  --space-3: 0.75rem;   /* 12px */
  --space-4: 1rem;      /* 16px */
  --space-5: 1.25rem;   /* 20px */
  --space-6: 1.5rem;    /* 24px */
  --space-8: 2rem;      /* 32px */
  --space-10: 2.5rem;   /* 40px */
  --space-12: 3rem;     /* 48px */
  --space-16: 4rem;     /* 64px */
}
```

### Spacing Rules

| Context | Value | Use |
|---------|-------|-----|
| Icon gap | `--space-2` | Icon + label pairs |
| Input padding | `--space-3` | Form controls |
| Card padding | `--space-4` | Standard cards |
| Section gap | `--space-6` | Between content blocks |
| Page margin | `--space-8` | Page-level padding |

---

## Depth & Elevation

**Strategy:** Borders-first with subtle shadows for floating elements.

```css
:root {
  /* Borders for structure */
  --border-width: 1px;
  --border-radius-sm: 4px;
  --border-radius-md: 6px;
  --border-radius-lg: 8px;
  --border-radius-xl: 12px;

  /* Shadows for floating elements only */
  --shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.3);
  --shadow-md: 0 4px 6px rgba(0, 0, 0, 0.4);
  --shadow-lg: 0 10px 15px rgba(0, 0, 0, 0.5);
  --shadow-glow: 0 0 20px rgba(59, 130, 246, 0.15);
}
```

### Elevation Hierarchy

| Level | Use | Treatment |
|-------|-----|-----------|
| 0 | Base canvas | `--bg-base` |
| 1 | Cards, panels | `--bg-surface` + `--border-default` |
| 2 | Dropdowns, popovers | `--bg-elevated` + `--shadow-md` |
| 3 | Modals, dialogs | `--bg-elevated` + `--shadow-lg` |

---

## Component Patterns

### Cards

```css
.card {
  background: var(--bg-surface);
  border: 1px solid var(--border-default);
  border-radius: var(--border-radius-lg);
  padding: var(--space-4);
}

.card:hover {
  background: var(--bg-hover);
  border-color: var(--border-strong);
}
```

### Buttons

```css
/* Primary - Blue, trust */
.btn-primary {
  background: var(--brand-primary);
  color: white;
  height: 36px;
  padding: 0 var(--space-4);
  border-radius: var(--border-radius-md);
  font-weight: var(--font-medium);
  transition: background 150ms ease;
}

.btn-primary:hover {
  background: var(--brand-hover);
}

/* Ghost - Subtle */
.btn-ghost {
  background: transparent;
  color: var(--fg-secondary);
  border: 1px solid var(--border-default);
}

.btn-ghost:hover {
  background: var(--bg-hover);
  color: var(--fg-primary);
}
```

### Data Tables

```css
.table {
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  font-variant-numeric: tabular-nums;
}

.table-header {
  color: var(--fg-muted);
  font-weight: var(--font-medium);
  text-transform: uppercase;
  letter-spacing: var(--tracking-wide);
  font-size: var(--text-xs);
}

.table-row:hover {
  background: var(--bg-hover);
}
```

### Order Book

```css
.orderbook-bid {
  background: linear-gradient(90deg, var(--success-subtle) var(--fill), transparent var(--fill));
}

.orderbook-ask {
  background: linear-gradient(270deg, var(--danger-subtle) var(--fill), transparent var(--fill));
}

.orderbook-price {
  font-family: var(--font-mono);
  font-weight: var(--font-semibold);
}
```

### Status Indicators

```css
.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.status-active { background: var(--success); }
.status-pending { background: var(--warning); }
.status-closed { background: var(--fg-muted); }
.status-private { background: var(--private); }
```

---

## Iconography

**Library:** Lucide Icons (consistent, MIT licensed)

**Size scale:**
- 16px - Inline with text
- 20px - Buttons, controls
- 24px - Section headers, navigation

**Style rules:**
- Stroke width: 1.5px (default Lucide)
- Color: Inherit from parent
- No filled variants (outline only)

---

## Motion

```css
:root {
  --duration-fast: 100ms;
  --duration-normal: 150ms;
  --duration-slow: 250ms;
  --ease-out: cubic-bezier(0.16, 1, 0.3, 1);
}

/* Respect user preference */
@media (prefers-reduced-motion: reduce) {
  * {
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }
}
```

### Motion Rules

| Interaction | Duration | Easing |
|-------------|----------|--------|
| Hover states | 150ms | ease-out |
| Button press | 100ms | ease-out |
| Modal open | 250ms | ease-out |
| Data updates | 150ms | ease-out |

---

## Signature Elements

### 1. Privacy Mode Indicator

When private mode is active, a subtle purple glow appears on the interface border:

```css
.app-shell[data-private="true"] {
  box-shadow: inset 0 0 0 1px var(--private);
}
```

### 2. Live Data Pulse

Real-time data updates trigger a subtle fade animation:

```css
@keyframes data-update {
  0% { background: var(--brand-subtle); }
  100% { background: transparent; }
}

.cell-updated {
  animation: data-update 500ms ease-out;
}
```

### 3. Order Book Depth Visualization

Horizontal fill bars showing liquidity at each price level - the signature of a professional trading interface.

### 4. Security Badge

A consistent shield icon with "Private" label appears when operating in encrypted mode.

---

## Layout Principles

### Information Density

Trading interfaces require high information density. Embrace it:
- Compact spacing (4px base)
- Smaller default font sizes (14px body)
- Multi-column layouts
- Persistent sidebars for navigation

### Grid System

```css
.dashboard-grid {
  display: grid;
  grid-template-columns: 280px 1fr 320px;
  gap: var(--space-4);
}

/* Sidebar - Markets/Navigation */
/* Center - Primary content (chart, order book) */
/* Right - Order entry, positions */
```

### Responsive Breakpoints

| Breakpoint | Layout |
|------------|--------|
| < 768px | Single column, bottom nav |
| 768-1024px | Two columns |
| 1024-1440px | Three columns |
| > 1440px | Three columns, wider margins |

---

## Accessibility

### Contrast Ratios

| Pairing | Ratio | Status |
|---------|-------|--------|
| `--fg-primary` on `--bg-base` | 15.8:1 | AAA |
| `--fg-secondary` on `--bg-base` | 7.2:1 | AAA |
| `--fg-muted` on `--bg-base` | 4.7:1 | AA |
| `--brand-primary` on `--bg-base` | 5.1:1 | AA |

### Focus States

```css
:focus-visible {
  outline: 2px solid var(--brand-primary);
  outline-offset: 2px;
}
```

### Keyboard Navigation

- All interactive elements focusable
- Tab order matches visual order
- Arrow key navigation in order book
- Escape closes modals

---

## Implementation Checklist

Before shipping any UI:

- [ ] Colors from system only (no hardcoded hex)
- [ ] Typography from scale only
- [ ] Spacing from scale only
- [ ] Monospace font for all numbers/prices
- [ ] Tabular figures enabled
- [ ] Hover states on all interactive elements
- [ ] Focus states visible
- [ ] Loading states for async content
- [ ] Error states for failed operations
- [ ] Empty states for no data
- [ ] Responsive at all breakpoints
- [ ] prefers-reduced-motion respected
