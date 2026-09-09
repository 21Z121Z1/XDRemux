# Apple Features

English | [简体中文](apple-features.md)

Photographic Styles and Apple Portrait are Rust-owned product intents in XDRemux. They are not a second product stack.

The canonical public product is the Rust `xdremux` CLI. The Swift target is only the Apple framework adapter for operations that cannot be performed portably, such as ImageIO consumer probing, Vision observations, and VideoToolbox encoding.

## Current availability

Standard HDR, OPPO-compatible output, Motion Photo → Live Photo, batch processing, categorization, inspection, and portable validation belong to the canonical Rust product.

Photographic Styles and Apple Portrait are expressed as `convert`/`batch` product intents. They do not create Apple-specific CLI subcommands, and low-level adapter or solver controls are not part of the public contract.

Rust owns the feature request/result models, routing, fallback, validation policy, metadata synthesis, assembly, and publication lifecycle. Apple-native code performs only the framework operations requested by Rust and returns factual observations.

## Platform boundary

The CLI uses this structure:

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

The CLI/runtime uses a versioned, bounded helper-process protocol. Each conversion reuses one lazily started adapter session. Timeout, crash, and framing failures terminate that session. Rust keeps transport private so that a transport change does not change engine or CLI semantics.

## Rust-owned policy

Rust already models the user-level Apple feature intent as two facts: Photographic Styles and Portrait. It does not expose the old Swift producer, donor, backend, or research controls as product configuration.

The first real Apple adapter operation is ImageIO auxiliary-resource probing. The adapter reports observations such as:

- ISO Gain Map presence;
- disparity presence;
- Portrait Effects Matte presence;
- skin, hair, teeth, and glasses semantic matte presence.

The adapter does not answer a business-level question such as “is this a valid portrait output?”. `xdremux-engine` owns that decision through `AppleImageAuxiliaryFacts` and the Portrait resource contract.

New operations must follow this boundary: return the narrowest useful framework fact or operation result, then keep policy in Rust.

## Photographic Styles migration

Rust owns style-generation semantics, constrained search, source-bound policy, key1/property-list synthesis, graph assembly, validation policy, and publication. The adapter is limited to framework observations or encoding primitives requested by the Rust runtime.

Research-only producers, model experiments, donor diagnostics, and RAW experiments remain research tooling. They do not define the public CLI contract or provide a second runtime.

## Apple Portrait migration

Rust owns Portrait preflight, OPPO block parsing, focus/orientation policy, JPEG/container logic, Gain Map policy, REND generation, auxiliary-manifest construction, feature routing, output naming, validation policy, and atomic publication. ImageIO reports auxiliary-resource facts and the adapter performs only the Apple framework operations required by the Rust transaction.

### Source-native profile and reduced effects

`convert` and `batch` accept `--apple-portrait-oppo`. This profile does not call Vision, including its private segmentation SPI. It uses public ImageIO and Core Image operations, so macOS is still required.

| Resource | Source-native profile |
| --- | --- |
| ISO Gain Map | Preserved from the source. |
| Disparity, focus, aperture and REND | Derived by Rust from OPPO depth, capture data and source metadata. |
| Portrait Effects Matte | Added only when the OPPO portrait plane has credible foreground. |
| Hair matte | Added only when the OPPO hair plane has credible foreground. |
| Skin, teeth and glasses mattes | Absent. No substitute masks are generated. |

Portrait effects are reduced compared with the complete `--apple-portrait` profile. Low-resolution or missing producer masks can reduce subject separation and hair detail. Semantic relighting and related edits can be limited. A depth-only output is valid for this source-native resource contract; it is not equivalent to the complete semantic resource set. An OPPO pet plane is not relabeled as a person or hair matte.

The native profile never falls back to Vision. Missing required depth or source metadata causes an error before output publication. The complete profile keeps its existing strict resource requirements. Both profiles use the same Rust conversion and publication path.

The native regression uses two real OPPO Portrait fixtures, checks single-file and serial/parallel batch output with ImageIO, and rejects any attempted Vision operation. It also verifies that a failed preflight preserves an existing output. Run it with `python3 scripts/check_rust_cli_oppo_portrait.py`.

## Live Photo

Normal Motion Photo → Live Photo conversion is already a Rust product capability and should not be routed through the Apple capability adapter.

If a future combined feature requires applying an Apple-only operation to a Live Photo still, Rust must continue to own the Live Photo asset lifecycle, pair identity, publication, and validation ordering. The Apple adapter should receive only the narrow platform operation it needs to perform.

## Compatibility rules

Product-level compatibility rules belong in Rust. Examples include whether OPPO-compatible output can be combined with an Apple editing feature, whether a source asset contains the resources required for Portrait editing, and how a combined request is published atomically.

Do not move these decisions into the adapter protocol when extending the CLI.

## Validation and acceptance

Use three distinct evidence classes:

1. Structural evidence proves that HEIF/MOV resources and metadata are present and parseable.
2. Native framework evidence proves that the tested Apple framework accepts or exposes the expected resources.
3. Device evidence proves behavior in a specific Apple Photos version on a real device.

Structural evidence does not replace device evidence for an interactive Apple Photos editing claim.

The canonical completion gate requires the Rust workspace and a real Rust → Apple adapter handshake on macOS. Feature-specific gates drive the Rust CLI and then query Apple consumer facts. Structural and native-framework evidence do not by themselves claim visual equivalence or Photos device acceptance.

CLI migration acceptance covers the canonical Rust commands, media conversion, source-dependent resource contracts and the Apple adapter. macOS app integration is a separate scope. Native-profile quality reduction is a documented product tradeoff. Interactive Photos editing remains a separate consumer claim; a CLI pass does not assert identical Photos effects or editing UI.

## Research material

The repository still contains style research code and optional models such as `ReverseKey1Ensemble`. These are research/training assets rather than product modes. See the [model card](../Models/ReverseKey1Ensemble.model-card.en.md) where relevant.
