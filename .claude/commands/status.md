---
name: polyguard-design:status
description: Show current design system state and available tools.
---

# Design System Status

Show current design system state.

## What to Show

**If `.design-system/system.md` exists:**

Display:
```
Design System: [Project Name]

Direction: [Precision & Density / Warmth / etc]
Foundation: [Cool slate / Warm stone / etc]
Depth: [Borders-only / Subtle shadows / Layered]

Tokens:
- Spacing base: 4px
- Radius scale: 4px, 6px, 8px
- Colors: [count] defined

Patterns:
- Button Primary (36px h, 16px px, 6px radius)
- Card Default (border, 16px pad)
- [other patterns...]

Last updated: [from git or file mtime]
```

**If no system.md:**

```
No design system found.

Options:
1. Run /polyguard-design:init to start building with craft principles
2. Run /polyguard-design:extract to pull patterns from existing code

Or generate a design system:
  python3 .claude/skills/polyguard-design/scripts/search.py "blockchain security dashboard" --design-system -p "Polyguard"
```

## Implementation

1. Read `.design-system/system.md`
2. Parse direction, tokens, patterns
3. Format and display
4. If no system, suggest next steps
