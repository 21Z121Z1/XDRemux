# Code-quality criteria

Look for behavior-preserving simplifications with a real reduction in maintained obligations. Prefer direct code with clear ownership. Apply these criteria as evidence-based review, not automatic file-size or style gates.

- Identify branches, lifecycle states, helpers, modes, or layers that can disappear while preserving required behavior.
- Trace the full producer/consumer path before calling code dead. Include public extension paths and generated or configuration-driven consumers.
- Prefer a model or state transition that makes the contract visible over scattered flags, casts, and silent fallback. Do not introduce a generic registry for one simple use.
- Reuse canonical helpers and owners. Check for business policy leaking into Swift, Python, scripts, wire types, or the app shell.
- Question wrappers that pass through one value without adding an owned contract, and caches or duplicated histories that can read an authoritative value directly.
- Inspect atomicity and recoverability when multiple filesystem or helper effects belong to one transaction. Parallelize only truly independent work; do not weaken order required by a native lifecycle or publication contract.
- Treat a large or growing file as a prompt to inspect cohesion and traceability. A line count is not itself a defect or a required extraction.

For a proposed restructure, name the current owner and consumers, what disappears, what remains, the behavior given up, and the strongest reason to retain the current design. Do not block a correct focused patch for speculative cleanup elsewhere. A meaningful maintained-complexity regression can be actionable when its actual cost and simpler alternative are concrete.
