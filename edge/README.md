# Neuraminds Closed Edge

This directory is reserved for closed-edge implementation.

Rules:
- No imports from `edge/` into open-core runtime modules.
- Closed-edge code can depend on open-core modules, never the reverse.
- Public releases can include stubs and interfaces here, but private logic stays internal.
