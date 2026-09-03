# Apple Features

English | [简体中文](apple-features.md)

Photographic Styles and Apple Portrait are Rust-owned product intents in XDRemux. They are not a second product stack.

The canonical public product is the Rust `xdremux` CLI. The Swift target is only the Apple framework adapter for operations that cannot be performed portably, such as ImageIO consumer probing, Vision observations, and VideoToolbox encoding.

## Current availability

Standard HDR, OPPO-compatible output, Motion Photo → Live Photo, batch processing, categorization, inspection, and portable validation belong to the canonical Rust product.

Photographic Styles and Apple Portrait are expressed as `convert`/`batch` product intents. They do not create Apple-specific CLI subcommands, and low-level adapter or solver controls are not part of the public contract.

Rust owns the feature request/result models, routing, fallback, validation policy, metadata synthesis, assembly, and publication lifecycle. Apple-native code performs only the framework operations requested by Rust and returns factual observations.

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

Rust owns style-generation semantics, constrained search, source-bound policy, key1/property-list synthesis, graph assembly, validation policy, and publication. The adapter is limited to framework observations or encoding primitives requested by the Rust runtime.

Research-only producers, model experiments, donor diagnostics, and RAW experiments remain research tooling. They do not define the public CLI contract or provide a second runtime.

## Apple Portrait migration

Rust owns Portrait preflight, OPPO block parsing, focus/orientation policy, JPEG/container logic, Gain Map policy, REND generation, auxiliary-manifest construction, feature routing, output naming, validation policy, and atomic publication. ImageIO reports auxiliary-resource facts and the adapter performs only the Apple framework operations required by the Rust transaction.

## Live Photo

Normal Motion Photo → Live Photo conversion is already a Rust product capability and should not be routed through the Apple capability adapter.

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

The canonical completion gate requires the Rust workspace and a real Rust → Apple adapter handshake on macOS. Feature-specific gates drive the Rust CLI and then query Apple consumer facts. Structural and native-framework evidence do not by themselves claim visual equivalence or Photos device acceptance.

## Research material

The repository still contains style research code and optional models such as `ReverseKey1Ensemble`. These are research/training assets rather than product modes. See the [model card](../Models/ReverseKey1Ensemble.model-card.en.md) where relevant.
