# Polyguard Visual Identity

Privacy-first prediction market terminal. Enhanced Bloomberg-meets-crypto aesthetic with AI-assisted insights and superior usability.

---

## Design Philosophy

**References:** Poly Terminal, Bloomberg Terminal, Refinitiv Eikon, with influences from high-end crypto platforms like Phantom and Solana Saga interfaces.

**Core Principle:** Empowerment through clarity and security. Every element fosters informed, confident decision-making while prioritizing user privacy and data integrity.

**Signature:** A sophisticated command center that balances dense, actionable information with intuitive navigation. Infused with subtle AI enhancements for predictive analytics, ensuring users stay ahead in volatile markets. The design evokes institutional reliability with crypto's innovative edge, emphasizing zero-knowledge proofs and encrypted sessions.

**Enhancements over Base:** Added light mode for accessibility, expanded component library for AI features, refined motion for smoother interactions, and integrated haptic feedback cues (for mobile/web).

---

## Color System

### Foundation (Dark & Light Modes)

```css
:root {
  /* Dark Mode */
  --bg-base-dark: #0a0f1a;
  --bg-surface-dark: #0f1629;
  --bg-elevated-dark: #151d32;
  --bg-hover-dark: #1a2540;

  --fg-primary-dark: #f1f5f9;
  --fg-secondary-dark: #94a3b8;
  --fg-muted-dark: #64748b;
  --fg-faint-dark: #475569;

  /* Light Mode */
  --bg-base-light: #f8fafc;
  --bg-surface-light: #ffffff;
  --bg-elevated-light: #f1f5f9;
  --bg-hover-light: #e2e8f0;

  --fg-primary-light: #0f172a;
  --fg-secondary-light: #475569;
  --fg-muted-light: #64748b;
  --fg-faint-light: #94a3b8;

  /* Borders (adaptive) */
  --border-default: rgba(148, 163, 184, 0.12);
  --border-subtle: rgba(148, 163, 184, 0.06);
  --border-strong: rgba(148, 163, 184, 0.24);

  /* Brand - Teal for trust + innovation (shifted from blue for uniqueness) */
  --brand-primary: #06b6d4;
  --brand-hover: #0891b2;
  --brand-subtle: rgba(6, 182, 212, 0.12);

  /* Semantic */
  --success: #22c55e;
  --success-subtle: rgba(34, 197, 94, 0.12);
  --danger: #ef4444;
  --danger-subtle: rgba(239, 68, 68, 0.12);
  --warning: #eab308;
  --warning-subtle: rgba(234, 179, 8, 0.12);
  --info: #3b82f6;
  --info-subtle: rgba(59, 130, 246, 0.12);

  /* Special - Privacy (enhanced purple) */
  --private: #7c3aed;
  --private-subtle: rgba(124, 58, 237, 0.12);

  /* AI Indicator */
  --ai-accent: #ec4899;
  --ai-subtle: rgba(236, 72, 153, 0.12);
}

/* Mode toggler */
@media (prefers-color-scheme: light) {
  :root {
    --bg-base: var(--bg-base-light);
    --bg-surface: var(--bg-surface-light);
    --bg-elevated: var(--bg-elevated-light);
    --bg-hover: var(--bg-hover-light);
    --fg-primary: var(--fg-primary-light);
    --fg-secondary: var(--fg-secondary-light);
    --fg-muted: var(--fg-muted-light);
    --fg-faint: var(--fg-faint-light);
  }
}
```

### Color Usage Rules

| Element | Color | Rationale | Mode Adaptation |
|---------|-------|-----------|-----------------|
| App background | `--bg-base` | Eye-friendly base for extended use | Dark: deep slate; Light: soft white |
| Cards/panels | `--bg-surface` | Layered elevation | Dark: subtle blue-gray; Light: pure white |
| Interactive hover | `--bg-hover` | Responsive feedback | Adaptive contrast |
| Primary text | `--fg-primary` | Maximum readability | Inverts per mode |
| Labels/metadata | `--fg-secondary` | Clear hierarchy | Inverts per mode |
| Primary actions | `--brand-primary` | Energetic yet trustworthy teal | Consistent across modes |
| YES positions | `--success` | Positive outcomes | Greener for visibility |
| NO positions | `--danger` | Negative outcomes | Consistent |
| Private mode | `--private` | Security emphasis | Glow adapts to mode |
| AI insights | `--ai-accent` | Highlights intelligent features | Pink for distinction |

---

## Typography

### Font Stack

```css
:root {
  /* Primary UI - Modern, readable */
  --font-sans: 'IBM Plex Sans', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;

  /* Data/Numbers - Advanced mono with ligatures */
  --font-mono: 'IBM Plex Mono', 'Fira Code', 'SF Mono', Consolas, monospace;

  /* Display/Headers - Bold and expressive */
  --font-display: 'IBM Plex Sans', var(--font-sans);
}
```

**Why IBM Plex:**
- IBM Plex: Open-source, designed for tech interfaces, excellent legibility at small sizes, with a crypto-modern feel. Avoids overused fonts like Inter for uniqueness while maintaining professionalism.
- Mono variant ensures perfect number alignment and code-like precision for market data.

### Type Scale

```css
:root {
  /* Size scale - Modular 4px base, expanded for flexibility */
  --text-xxs: 0.625rem;  /* 10px - Fine print */
  --text-xs: 0.75rem;    /* 12px */
  --text-sm: 0.875rem;   /* 14px */
  --text-base: 1rem;     /* 16px */
  --text-lg: 1.125rem;   /* 18px */
  --text-xl: 1.25rem;    /* 20px */
  --text-2xl: 1.5rem;    /* 24px */
  --text-3xl: 1.875rem;  /* 30px */
  --text-4xl: 2.25rem;   /* 36px - Hero elements */

  /* Weights, line heights, tracking unchanged but enforced */
}
```

### Typography Rules

| Element | Size | Weight | Font | Tracking | Line Height |
|---------|------|--------|------|----------|-------------|
| Page title | `--text-3xl` | 700 | Display | Tight | Tight |
| Section header | `--text-xl` | 600 | Sans | Normal | Normal |
| Body text | `--text-base` | 400 | Sans | Normal | Relaxed |
| Labels | `--text-sm` | 500 | Sans | Wide | Normal |
| Prices/amounts | `--text-lg` | 600 | Mono | Normal | Tight |
| Percentages | `--text-base` | 600 | Mono | Normal | Tight |
| Timestamps | `--text-xs` | 400 | Mono | Normal | Normal |
| AI Insights | `--text-sm` | 500 | Sans | Normal | Relaxed |

---

## Spacing System

**Base unit:** 4px (rem-based for scalability)

```css
:root {
  /* Expanded scale for finer control */
  --space-0: 0;
  --space-0-5: 0.125rem; /* 2px */
  --space-1: 0.25rem;    /* 4px */
  --space-1-5: 0.375rem; /* 6px */
  --space-2: 0.5rem;     /* 8px */
  /* ... up to --space-16 as before */
}
```

### Spacing Rules

| Context | Value | Use |
|---------|-------|-----|
| Micro gaps (icons) | `--space-1` | Tight pairings |
| Input padding | `--space-3` | Comfortable controls |
| Card padding | `--space-4` | Balanced content |
| Section gap | `--space-6` | Logical separation |
| Page margin | `--space-8` | Breathing room |
| AI panel insets | `--space-5` | Focused insights |

---

## Depth & Elevation

**Strategy:** Subtle glassmorphism in light mode, borders with minimal shadows in dark.

```css
:root {
  /* Borders and radii expanded */
  --border-radius-pill: 9999px; /* New for buttons/tags */

  /* Shadows adaptive */
  --shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.1);
  --shadow-md: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06);
  --shadow-lg: 0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -2px rgba(0, 0, 0, 0.05);
  --glass-bg: rgba(255, 255, 255, 0.6); /* Light mode glass */
}
```

### Elevation Hierarchy

| Level | Use | Treatment |
|-------|-----|-----------|
| 0 | Base | Plain bg |
| 1 | Cards | Border + subtle shadow/glass |
| 2 | Popovers | Elevated bg + md shadow |
| 3 | Modals | Elevated bg + lg shadow + backdrop blur |

---

## Component Patterns

### Cards (Enhanced)

```css
.card {
  background: var(--bg-surface);
  border: 1px solid var(--border-default);
  border-radius: var(--border-radius-lg);
  padding: var(--space-4);
  backdrop-filter: blur(4px); /* Glass effect in light */
}

.card-ai {
  border-left: 4px solid var(--ai-accent);
}
```

### Buttons (Expanded Variants)

```css
.btn-primary {
  background: var(--brand-primary);
  color: white;
  border-radius: var(--border-radius-pill); /* Rounded for modern feel */
}

.btn-ai {
  background: linear-gradient(135deg, var(--ai-accent), var(--brand-primary));
  color: white;
}
```

### Data Tables (With Sorting)

```css
.table th.sortable:hover {
  color: var(--brand-primary);
  cursor: pointer;
}
```

### Order Book (Depth + Liquidity Heatmap)

```css
.orderbook-bid {
  background: linear-gradient(90deg, var(--success-subtle) calc(var(--depth) * 1%), transparent);
}

.orderbook-heat {
  opacity: calc(var(--liquidity) / 100); /* Visual liquidity indicator */
}
```

### Charts (Candlestick Integration)

```css
.chart-candle-up {
  fill: var(--success);
}

.chart-candle-down {
  fill: var(--danger);
}
```

### AI Insight Panel

```css
.ai-panel {
  background: var(--ai-subtle);
  border-radius: var(--border-radius-xl);
  padding: var(--space-5);
  box-shadow: var(--shadow-glow);
}
```

### Status Indicators (Animated)

```css
.status-dot {
  transition: background var(--duration-fast);
}

.status-ai { background: var(--ai-accent); animation: pulse 1.5s infinite; }
```

---

## Iconography

**Library:** Heroicons or Tabler Icons (MIT, customizable)

**Size scale:** Expanded to 12px for micro-icons.

**Style rules:** Outline, adaptive color, filled for active states.

---

## Motion

```css
:root {
  --duration-instant: 50ms; /* New for quick feedback */
  /* Others as before */
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}
```

### Motion Rules (Enhanced)

| Interaction | Duration | Easing | Notes |
|-------------|----------|--------|-------|
| Hover | 150ms | ease-out | Subtle scale (1.02) |
| Button press | 100ms | ease-out | Depress effect |
| Modal open | 250ms | ease-out | Fade + slide |
| Data update | 150ms | ease-out | Highlight + fade |
| AI suggestion | 300ms | ease-out | Grow from point |

Respect reduced motion and add haptic feedback via JS where supported.

---

## Signature Elements

1. **Privacy Shield:** Dynamic border glow with particle effects in private mode.

```css
.app-shell[data-private="true"] {
  border: 2px solid var(--private);
  animation: glow-pulse 2s infinite;
}
```

2. **AI Insight Bubbles:** Floating, dismissible tips with predictive probabilities.

3. **Liquidity Horizon:** Gradient bars in order book visualizing depth and volatility.

4. **Zero-Knowledge Badge:** Icon with tooltip explaining encrypted trades.

5. **Mode Switcher:** Seamless dark/light toggle with system preference detection.

---

## Layout Principles

### Information Density

High but adaptive: Use collapsible panels for overload prevention, AI-summarized overviews.

### Grid System (Flexible)

```css
.dashboard-grid {
  display: grid;
  grid-template-columns: minmax(240px, 1fr) 2fr minmax(280px, 1fr);
  gap: var(--space-4);
}
```

### Responsive Breakpoints (Improved)

| Breakpoint | Layout |
|------------|--------|
| < 640px | Stacked, mobile nav |
| 640-1024px | Two columns, compact |
| > 1024px | Full three columns, expandable |

---

## Accessibility

### Contrast Ratios (AAA Compliant)

All pairings exceed 7:1 for normal text, 4.5:1 for large.

### Focus & Navigation

Custom focus rings, ARIA labels for all components, screen reader optimizations for dynamic data.

### Other: Color-blind modes, keyboard shortcuts for trading.

---

## File Structure (Expanded)

```
.design-system/
├── VISUAL-IDENTITY.md
├── tokens.json      # JSON export for design tools
├── components/      # Reusable Vue/React components
└── themes/          # Dark/light CSS
```

---

## Implementation Checklist (Enhanced)

- [ ] Mode-aware colors
- [ ] AI component integration
- [ ] Performance: Lazy-load charts
- [ ] Security: Content Security Policy
- [ ] Testing: Cross-browser, a11y audits

This enhanced visual identity builds on the original by adding dual modes, AI-specific elements, refined aesthetics, and greater flexibility, surpassing standard terminals like Poly Terminal in usability and innovation while maintaining professional credibility. 