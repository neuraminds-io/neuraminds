---
name: polyguard-design:init
description: Build UI with craft and consistency. Unified design intelligence for all interface types.
---

## Required Reading

Before writing any code, read these files:

1. `.claude/skills/polyguard-design/SKILL.md` - Foundation and workflow
2. `.claude/skills/polyguard-design/references/principles.md` - Craft principles and code examples
3. `.claude/skills/polyguard-design/references/example.md` - How decisions translate to code

---

**Scope:** All UI work - dashboards, admin panels, SaaS apps, landing pages, e-commerce, tools.

## Step 1: Intent First

Before touching code, answer these:

**Who is this human?** Not "users." Where are they? What's on their mind?

**What must they accomplish?** Not "use the dashboard." The verb.

**What should this feel like?** Words that mean something. "Clean" means nothing. Warm like a notebook? Cold like a terminal?

If you cannot answer with specifics, stop and ask the user.

## Step 2: Generate Design System

Run the design system generator:

```bash
python3 .claude/skills/polyguard-design/scripts/search.py "<product_type> <industry> <keywords>" --design-system -p "Project Name"
```

Example for Polyguard:
```bash
python3 .claude/skills/polyguard-design/scripts/search.py "blockchain security dashboard dark professional" --design-system -p "Polyguard"
```

## Step 3: Domain Exploration (as needed)

Get detailed recommendations:

```bash
# Style options
python3 .claude/skills/polyguard-design/scripts/search.py "dark security" --domain style

# Typography
python3 .claude/skills/polyguard-design/scripts/search.py "professional technical" --domain typography

# UX guidelines
python3 .claude/skills/polyguard-design/scripts/search.py "dashboard accessibility" --domain ux
```

## Step 4: Stack Guidelines

```bash
python3 .claude/skills/polyguard-design/scripts/search.py "<keyword>" --stack <stack-name>
```

Available: `html-tailwind`, `react`, `nextjs`, `vue`, `svelte`, `shadcn`, `flutter`, `swiftui`, `react-native`

## Before Writing Each Component

State intent AND technical approach:

```
Intent: [who, what they need to do, how it should feel]
Palette: [colors and WHY]
Depth: [borders / shadows / layered and WHY]
Surfaces: [elevation scale and WHY]
Typography: [typeface choice and WHY]
Spacing: [base unit]
```

## Flow

1. Read required files
2. Check if `.design-system/system.md` exists
3. **If exists**: Apply established patterns
4. **If not**: Assess context, suggest direction, get confirmation, build

## After Every Task

Offer to save:

"Want me to save these patterns to `.design-system/system.md`?"
