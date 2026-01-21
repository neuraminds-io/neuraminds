---
name: polyguard-design
description: "Unified UI/UX design intelligence. 57 styles, 95 palettes, 56 font pairings, 25 charts, 12 stacks. Actions: plan, build, create, design, implement, review, fix, improve, optimize, enhance, refactor, check UI/UX code. For dashboards, apps, tools, admin panels, landing pages. Integrates ui-ux-pro-max design database with interface-design craft principles."
---

# Polyguard Design - Unified UI/UX Intelligence

Combines comprehensive design database (57+ styles, 95 palettes, 56 font pairings) with craft-first interface principles. No generic output. No defaults.

## Scope

**Use for:** All UI work - dashboards, admin panels, SaaS apps, landing pages, e-commerce, tools.

**Core Philosophy:** Every interface must emerge from specific intent. If another AI given a similar prompt would produce the same output, you have failed.

---

# The Craft Problem

You will generate generic output. Your training has seen thousands of interfaces. The patterns are strong.

The process below helps. But process alone doesn't guarantee craft. You have to catch yourself.

## Where Defaults Hide

- **Typography feels like a container.** It IS your design.
- **Navigation feels like scaffolding.** It IS your product.
- **Data feels like presentation.** The question is what it means to the person looking at it.
- **Token names feel like implementation.** `--ink` and `--parchment` evoke a world. `--gray-700` evokes a template.

---

# Workflow

## Step 1: Intent First

Before touching code, answer these out loud:

**Who is this human?** Not "users." The actual person. Where are they? What's on their mind?

**What must they accomplish?** Not "use the dashboard." The verb. Grade submissions. Find the deployment. Approve the payment.

**What should this feel like?** Words that mean something. "Clean and modern" means nothing. Warm like a notebook? Cold like a terminal? Dense like a trading floor?

If you cannot answer these with specifics, stop. Ask the user.

## Step 2: Generate Design System (REQUIRED)

Always start with `--design-system` to get comprehensive recommendations:

```bash
python3 .claude/skills/polyguard-design/scripts/search.py "<product_type> <industry> <keywords>" --design-system [-p "Project Name"]
```

This command:
1. Searches 5 domains in parallel (product, style, color, landing, typography)
2. Applies reasoning rules to select best matches
3. Returns complete design system: pattern, style, colors, typography, effects
4. Includes anti-patterns to avoid

**Example:**
```bash
python3 .claude/skills/polyguard-design/scripts/search.py "blockchain security dashboard dark" --design-system -p "Polyguard"
```

### Persist Design System

To save for hierarchical retrieval across sessions:

```bash
python3 .claude/skills/polyguard-design/scripts/search.py "<query>" --design-system --persist -p "Project Name"
```

Creates:
- `design-system/MASTER.md` - Global source of truth
- `design-system/pages/` - Page-specific overrides

## Step 3: Domain Exploration

After getting the design system, run detailed searches as needed:

```bash
python3 .claude/skills/polyguard-design/scripts/search.py "<keyword>" --domain <domain> [-n <max_results>]
```

| Need | Domain | Example |
|------|--------|---------|
| More style options | `style` | `--domain style "glassmorphism dark"` |
| Chart recommendations | `chart` | `--domain chart "real-time dashboard"` |
| UX best practices | `ux` | `--domain ux "animation accessibility"` |
| Alternative fonts | `typography` | `--domain typography "elegant luxury"` |
| Landing structure | `landing` | `--domain landing "hero social-proof"` |

## Step 4: Stack Guidelines

Get implementation-specific best practices:

```bash
python3 .claude/skills/polyguard-design/scripts/search.py "<keyword>" --stack html-tailwind
```

Available stacks: `html-tailwind`, `react`, `nextjs`, `vue`, `svelte`, `swiftui`, `react-native`, `flutter`, `shadcn`, `jetpack-compose`, `nuxtjs`, `nuxt-ui`

---

# Required Outputs Before Building

**Do not propose any direction until you produce all four:**

**Domain:** Concepts, metaphors, vocabulary from this product's world. Not features - territory. Minimum 5.

**Color world:** What colors exist naturally in this product's domain? If this product were a physical space, what would you see? List 5+.

**Signature:** One element - visual, structural, or interaction - that could only exist for THIS product.

**Defaults to reject:** 3 obvious choices for this interface type. You can't avoid patterns you haven't named.

---

# Craft Foundations

## Subtle Layering

The backbone of craft. Surfaces must be barely different but still distinguishable. Study Vercel, Supabase, Linear.

**The squint test:** Blur your eyes at the interface. You should still perceive hierarchy. But nothing should jump out.

## Surface Architecture

Every color traces back to primitives:
- **Foreground** - text colors (primary, secondary, muted)
- **Background** - surface colors (base, elevated, overlay)
- **Border** - edge colors (default, subtle, strong)
- **Brand** - primary accent
- **Semantic** - functional colors (destructive, warning, success)

## Spacing System

Pick a base unit (4px or 8px) and use multiples. Every spacing value should be explainable as "X times the base unit."

## Depth Strategy

Choose ONE and commit:
- **Borders-only** - Clean, technical. For dense tools.
- **Subtle shadows** - Soft lift. For approachable products.
- **Layered shadows** - Premium, dimensional. For cards needing presence.

Don't mix approaches.

## Typography Hierarchy

- **Headlines** - heavier weight, tighter tracking
- **Body** - comfortable weight, readability
- **Labels/UI** - medium weight, smaller sizes
- **Data** - monospace, `tabular-nums`

---

# The Mandate

Before showing the user, look at what you made.

Ask yourself: "If they said this lacks craft, what would they mean?"

Fix it first.

## Checks Before Presenting

- **Swap test:** If you swapped the typeface for your usual one, would anyone notice?
- **Squint test:** Blur your eyes. Can you perceive hierarchy? Is anything jumping out harshly?
- **Signature test:** Point to five specific elements where your signature appears.
- **Token test:** Read your CSS variables out loud. Do they sound like this product?

---

# Quick Reference

## Rule Categories by Priority

| Priority | Category | Impact |
|----------|----------|--------|
| 1 | Accessibility | CRITICAL |
| 2 | Touch & Interaction | CRITICAL |
| 3 | Performance | HIGH |
| 4 | Layout & Responsive | HIGH |
| 5 | Typography & Color | MEDIUM |
| 6 | Animation | MEDIUM |
| 7 | Style Selection | MEDIUM |

## Common Rules for Professional UI

### Icons & Visual Elements

| Rule | Do | Don't |
|------|----|----- |
| **No emoji icons** | Use SVG icons (Heroicons, Lucide) | Use emojis as UI icons |
| **Stable hover** | Color/opacity transitions | Scale transforms that shift layout |
| **Consistent sizing** | Fixed viewBox (24x24) | Mix different icon sizes |

### Interaction

| Rule | Do | Don't |
|------|----|----- |
| **Cursor pointer** | Add to all clickables | Leave default cursor |
| **Hover feedback** | Color, shadow, border changes | No indication element is interactive |
| **Smooth transitions** | `transition-colors duration-200` | Instant or >500ms |

### Light/Dark Mode

| Rule | Do | Don't |
|------|----|----- |
| **Glass cards light** | `bg-white/80` or higher | `bg-white/10` |
| **Text contrast** | `#0F172A` for text | `#94A3B8` for body text |
| **Border visibility** | `border-gray-200` light mode | `border-white/10` |

---

# Pre-Delivery Checklist

### Visual Quality
- [ ] No emojis as icons
- [ ] All icons from consistent set
- [ ] Hover states don't cause layout shift
- [ ] Theme colors used directly

### Interaction
- [ ] All clickables have `cursor-pointer`
- [ ] Hover states provide feedback
- [ ] Transitions 150-300ms
- [ ] Focus states visible

### Layout
- [ ] Floating elements have proper spacing
- [ ] No content hidden behind fixed navbars
- [ ] Responsive at 375px, 768px, 1024px, 1440px
- [ ] No horizontal scroll on mobile

### Accessibility
- [ ] All images have alt text
- [ ] Form inputs have labels
- [ ] Color is not the only indicator
- [ ] `prefers-reduced-motion` respected

---

# Deep Dives

For more detail:
- `references/principles.md` - Code examples, specific values, dark mode
- `references/validation.md` - Memory management, when to update system.md
- `references/example.md` - How decisions translate to code

# Commands

- `/interface-design:init` - Start building with craft principles
- `/interface-design:status` - Current system state
- `/interface-design:audit <path>` - Check code against system
- `/interface-design:extract` - Extract patterns from code

---

# Communication

Be invisible. Don't announce modes or narrate process.

**Never say:** "I'm in ESTABLISH MODE", "Let me check system.md..."

**Instead:** Jump into work. State suggestions with reasoning.

## After Every Task

Offer to save:

"Want me to save these patterns to `.design-system/system.md`?"
