# Apple Features

English | [简体中文](apple-features.md)

Photographic Styles and Apple Portrait are the remaining major Apple-specific ownership migration in XDRemux. They are not a second product stack.

The canonical public product is the Rust `xdremux` CLI. The legacy Swift Apple implementation remains only as a migration oracle and as the current implementation of platform capabilities that still require ImageIO, Core Image, Vision, Core ML, AVFoundation, or other Apple frameworks.

## Current availability

Standard HDR, OPPO-compatible output, Motion Photo → Live Photo, batch processing, categorization, inspection, and portable validation belong to the canonical Rust product.

Photographic Styles and Apple Portrait are not yet exposed as stable commands/options by the canonical Rust CLI. Do not use legacy Swift-only switches as a specification for new product behavior.

The migration is complete only when Rust owns the feature request/result models, routing, fallback, validation policy, and publication lifecycle, while Apple-native code performs only the framework operations requested by Rust.

## Platform boundary

The target structure is:

```text
xdremux CLI
    ↓
Rust runtime
    ↓
Rust engine policy
    ↓
portable providers + Apple platform adapter
                         ↓
          ImageIO / Core Image / Vision /
          Core ML / AVFoundation / ...
```

`xdremux-apple-adapter` is a distributable platform component consumed by the Rust product. It is not a user CLI and does not own product policy.

The CLI/runtime implementation currently uses a versioned, bounded helper-process protocol. A sandboxed macOS app may use XPC when separate entitlements, sandboxing, lifecycle, or crash isolation are required. Transport is intentionally private to the runtime so that changing helper transport does not change engine or CLI semantics.

## Rust-owned policy

Rust already models the user-level Apple feature intent as two facts: Photographic Styles and Portrait. It does not expose the old Swift producer, donor, backend, or research controls as product configuration.

The first real Apple adapter operation is ImageIO auxiliary-resource probing. The adapter reports observations such as:

- ISO Gain Map presence;
- disparity presence;
- Portrait Effects Matte presence;
- skin, hair, teeth, and glasses semantic matte presence.

The adapter does not answer a business-level question such as “is this a valid portrait output?”. `xdremux-engine` owns that decision through `AppleImageAuxiliaryFacts` and the Portrait resource contract.

This pattern should be used for the remaining migration: return the narrowest useful framework fact or operation result, then keep policy in Rust.

## Photographic Styles migration

The legacy Swift implementation still contains substantial style-generation and Apple-framework behavior, including semantic analysis, Core ML paths, constrained style-data generation, and Apple-specific consumer validation.

Do not mechanically port its command-line controls or internal producer selection into Rust. The migration should instead split the implementation into narrowly scoped platform operations, for example framework analysis or model execution, while Rust owns feature routing and product defaults.

Research-only producers, model experiments, donor diagnostics, and RAW experiments remain research tooling. They do not define the public CLI contract.

## Apple Portrait migration

The legacy Swift Portrait pipeline still performs Apple-framework decoding/writing and contains policy that is being removed from Swift.

The first policy slice has already moved conceptually to Rust: ImageIO reports auxiliary-resource facts, and Rust decides whether the complete resource set satisfies the Portrait editing contract.

Continue migrating Portrait in the same direction. OPPO block parsing, JPEG/container logic, Gain Map policy, feature routing, output naming, and validation policy belong in Rust. Only operations that genuinely require Apple frameworks remain in the Apple capability layer.

## Live Photo

Normal Motion Photo → Live Photo conversion is already a Rust product capability and should not be routed back through the legacy Swift Apple feature engine.

If a future combined feature requires applying an Apple-only operation to a Live Photo still, Rust must continue to own the Live Photo asset lifecycle, pair identity, publication, and validation ordering. The Apple adapter should receive only the narrow platform operation it needs to perform.

## Compatibility rules

Product-level compatibility rules belong in Rust. Examples include whether OPPO-compatible output can be combined with an Apple editing feature, whether a source asset contains the resources required for Portrait editing, and how a combined request is published atomically.

Do not encode those decisions into the adapter protocol merely because the old Swift implementation currently makes them.

## Validation and acceptance

Use three distinct evidence classes:

1. Structural evidence proves that HEIF/MOV resources and metadata are present and parseable.
2. Native framework evidence proves that the tested Apple framework accepts or exposes the expected resources.
3. Device evidence proves behavior in a specific Apple Photos version on a real device.

Structural evidence does not replace device evidence for an interactive Apple Photos editing claim.

The canonical completion gate requires the Rust workspace and a real Rust → Apple adapter handshake on macOS. Feature-specific replacement gates should be added as Apple operations migrate. Keep the legacy Swift implementation only until the corresponding replacement evidence is complete.

## Research material

The repository still contains style research code and optional models such as `ReverseKey1Ensemble`. These are research/training assets rather than product modes. See the [model card](../Models/ReverseKey1Ensemble.model-card.en.md) where relevant.
