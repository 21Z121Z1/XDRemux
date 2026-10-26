# Code-quality criteria

Look for changes that preserve required behavior and reduce maintained obligations. Prefer direct code with clear ownership. Apply these criteria to evidence. File size and preferred style are not automatic gates.

## Inspect the maintained behavior

- Identify branches, lifecycle states, helpers, modes, or layers that can disappear without losing required behavior.
- Before calling code dead, trace all producers and consumers. Include public extension paths and consumers selected through generated code or configuration.
- Prefer models and state transitions that expose the contract. Inspect scattered flags, casts, and silent fallbacks for hidden state. Do not introduce a generic registry for one simple use.
- Reuse canonical helpers and owners. Check whether business policy has moved into Swift, Python, scripts, wire types, or the app shell.
- Question wrappers that only pass a value through and add no owned contract. Check whether caches or duplicated histories can read an authoritative value directly.
- If several filesystem or helper effects form one transaction, inspect atomicity and recovery. Parallelize only independent work. Preserve order required by native lifecycles and publication contracts.
- Use a large or growing file as a reason to inspect cohesion and traceability. Line count alone is not a defect or a requirement to extract code.

## Explain a proposed restructure

Identify the current owner and consumers. State what disappears, what remains, and which behavior the proposal gives up. Give the strongest reason to keep the current design.

Do not block a correct, focused patch for speculative cleanup elsewhere. A material increase in maintained complexity can justify a finding. Show the actual cost and a concrete, simpler alternative.
