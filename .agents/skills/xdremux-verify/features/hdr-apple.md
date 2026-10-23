# HDR and Apple editing

Verify HDR resource geometry and the actual Apple feature or app consumer reached by a change. Keep portable, native, and device results separate.

## Sub-features

Check HDR reconstruction and coded/display orientation; complete Styles/Texture or Portrait resources; Cinematic pending-state recognition; the macOS app's same product switch; and publication preservation on required failures.

## How to get to it (user POV)

Use `xdremux convert --input INPUT --output NEW_OUTPUT` for standard HDR. Add the documented `--apple-styles`, `--apple-portrait`, `--apple-portrait-oppo`, or `--apple-cinematic` intent for the corresponding feature. Exercise the app switch separately when app behavior changed.

## Driving it with existing repository gates

Preconditions: identify required media/source resources and the applicable OS/SDK/helper before running. Use fresh outputs. Read current Apple feature documentation for compatibility and hardware requirements.

- For orientation, run `cargo test --locked -p xdremux-codec --test orientation_roundtrip -- --nocapture`. Its real Samsung check is portable codec evidence. Use the existing product CI's `scripts/apple/read_oriented_heif_imageio.swift` step for ImageIO readback of the emitted artifact. Rotation/mirror association order and coded dimensions must remain meaningful, not just parseable.
- For Styles and HDR thumbnails, use `bash scripts/apple/check_rust_style_consumer.sh`. For Texture, run `python3 scripts/apple/check_texture_styles.py --all-fixtures --output NEW_EVIDENCE_DIRECTORY` on the required macOS 27 host. The latter covers convert, batch, app bridge, feature validation, media preservation, and an injected failure that preserves an existing output.
- For Portrait, use `bash scripts/apple/check_rust_cli_apple_portrait.sh` or `python3 scripts/apple/check_rust_cli_oppo_portrait.py` for the affected profile. Do not silently substitute one profile for the other.
- For Cinematic, use the current `docs/cinematic.en.md` procedure and existing `scripts/apple/verify_cinematic_envelope.py`; retain pending-state/native preprocessing differences. For app changes, use `apps/macos/XDRemuxApp/scripts/verify_model_tests.sh` plus the affected actual app entry point.
- For the optional system Gain Map experiment, reuse its current research job and `scripts/apple/measure_system_heif_hdr.sh`. A measurement failure before Styles publication must stay a failure; do not classify the prototype as the default product path.

## Gotchas

Linux excludes macOS-gated modules and cannot prove their native execution. A helper mock can prove Rust transport/policy only. A parser-visible URI or nonempty dictionary does not prove role truth, GPU success, final edit state, or Photos behavior. A generic adapter handshake does not prove private selectors or device resources exist. Final readback and failure-safe publication remain required even for typed internal media values. Real Photos import, parameter response, edit/save/reopen, display rendering, and device-specific quality require their own recorded device evidence.
