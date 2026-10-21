# Tone extractor: iinf v1 count mismatch

The real c379 first fault is now known. It is the fixed consumer's Styles item lookup at `extract_inputs.py:50`, after `tools/heif_dump.py` silently loses every `infe` declaration in a legal `iinf v1`. The log does not demonstrate that the product discarded a Styles URI or `cdsc` relationship. A real Rust Texture-writer structural reproduction preserves both and reproduces the failed lookup with the complete fixed parser.

The portable research-boundary repair is implemented and frozen in two files. Native Tone rendering still requires a new source-head run; this Linux result is not native acceptance or Photos Tone/Color admission.

## Exact failure and source

`/workspace/xdremux-audit/c379aa7-tone-job118944009628.log:1485–1507` records the LHDR failure; lines 1514–1523 and 1574–1584 record the equivalent failures for UHDR HR and portrait. All fail at the Styles metadata lookup, before the Linear Thumbnail or Style Delta resource lookups and before rendering. The upstream sweep still returns process code 0, but the local missing-data gate correctly fails and retains all three tracebacks.

Fixed upstream revision: `37dd551a949047ed14580efa0c0a2556922791b5`. Complete parser snapshot: `/workspace/xdremux-audit/tone-pinned-heif-dump.py` (28,948 bytes), SHA256 `24ef91b177d6346fb3952036e8e2b770c4491c7aad5bea85244a768794336e10`, Git blob `d65da02fb2ffadc334d4d00199d063d26bf7eb94`. Source: `tools/heif_dump.py`, `parse_iinf` at lines 178–202. It reads a two-byte entry count for both versions 0 and 1; version 1 requires four bytes. For the small legal v1 counts used here, the first two bytes are zero, so the function returns no item declarations. `parse_iloc` can still populate item IDs and `parse_iref` still supplies relationships, leaving the Styles lookup without `uri_type`.

The product source at exact c379 preserves source item-info boxes and payloads (`crates/xdremux-heif/src/texture.rs:126–150`), clones original references (`:126–130`), then emits `make_iinf_box(1, ...)` at line 393. The preceding Styles appender authors the Styles URI and its primary/tmap `cdsc` edge (`styles.rs:250–267,578–610`). No product metadata version or role change is needed to repair this parser width mismatch.

## Real writer RED and recovery

`tone-iinf-probe/crates/{xdremux-heif,xdremux-format}` is a source snapshot from `git archive c379aa7c4d1f6fcad8d2893837ee20e4ae7dd1d5`, plus an external probe manifest and lock. The probe calls the real public `assemble_texture_styles_heif` on a small structural fixture containing a primary, a Styles URI item, a linked Styles payload and `cdsc` edge. Its sample payload is synthetic and is not a real image, native render, HDR reference, or forged capture fact.

Command (independent target; no shared source edits):

```sh
env PATH=/tmp/xdremux-toolchain/cargo/bin:$PATH \
  CARGO_HOME=/tmp/xdremux-toolchain/cargo \
  RUSTUP_HOME=/tmp/xdremux-toolchain/rustup \
  CARGO_TARGET_DIR=/tmp/xdremux-tone-iinf-probe-target \
  cargo run --offline --manifest-path /workspace/xdremux-audit/tone-iinf-probe/Cargo.toml \
  -- /workspace/xdremux-audit/tone-iinf-probe/outputs
python3 /workspace/xdremux-audit/tone-iinf-probe/verify_parser.py
```

Rust output (`build-run.log`) confirms 2 source items → 3 Texture output items, original `cdsc 2→1` still present, Styles payload still 24 bytes. The complete pinned parser and final production `patched_iinf_parser` are compared in `complete-parser-proof.json` / `.log`:

| Bytes parsed | iinf version | Original complete parser | Final compatibility parser |
| --- | ---: | --- | --- |
| Before Texture | 0 | 2 declarations; 1 linked Styles item | 2 declarations; 1 linked Styles item |
| After real Texture writer | 1 | 0 declarations; 0 linked Styles items; original cdsc remains | 3 declarations; 1 linked Styles item; original cdsc remains |

The recovered Styles payload SHA256 is `539818dcc368fd093ffdc5c520c93f5aa80383305106f8c79fefab6e517331d5`, exactly equal before/after. Both structural input byte arrays and the pinned parser file remain unchanged. The full-parser verification explicitly rereads and checks them. `parser-red-recovery.log/.json` additionally preserve the original one-line width-only recovery before the guarded production adapter was implemented.

The actual macOS job output media is not available locally and was not downloaded or uploaded. This experiment proves the concrete producer/parser mechanism on real writer output; final confirmation on the three job products remains the new native run.

## Small local boundary repair

Only `scripts/apple/check_style_tone_response.py` and `tests/test_style_tone_response.py` changed. `run_style_tone_sweep.sh`, workflow, product Rust/Swift/HEIF, upstream checkout, metadata version, native Texture semantics and all comparison/math gates remain unchanged.

`patched_iinf_parser` checks the complete source SHA256 before touching anything, then requires every exact expected original fragment once within `parse_iinf`. It corrects the count width and rejects truncated headers/counts/children, unsupported iinf versions, incomplete counts, missing item IDs and duplicates. All other fixed parser functions remain original. The receipt records fixed revision, actual original Git blob/SHA256, patch SHA256, derived source SHA256 and launcher SHA256.

Stable repair identifiers:

- Patch specification SHA256: `c530cb377a50b1d7553e5633ec1f9a6eb148af2f1e366148814556de82a5c633`.
- Complete derived parser SHA256: `72a70eeed36098b6acd72677129db190839c4216c35a0f992202bfc1345a099c`.

`prepare_consumer_compatibility` creates only an owned `consumer-compat` directory under this invocation's fresh output directory. It holds an ephemeral derived parser and a small extraction launcher. The launcher rechecks original/derived parser hashes before loading the derived parser and executing the **original** extractor. No upstream file is overwritten. The original sweep script still runs; its native source and Metal model resolve from `__file__` at their fixed original paths. Only the sweep's cwd-based extractor path is routed through this owned launcher; inputs are converted to absolute paths. Parent independently retained and reviewed the exact sweep source at `/workspace/xdremux-audit/tone-pinned-run-sweep.py` (SHA256 `ac6c5d93a78ada33b95980ecb7f862559446dc20952129116f2bf08a9af7ec72`).

Hash mismatch fails before launch with a persisted failed receipt. Any downstream extraction/render failure still produces its real stderr, row failures and missing-measurement rejection. The existing upload whitelist admits only typed receipts and filtered text logs; the owned source copy/launcher, dictionaries, resources and media are not uploaded.

## Tests and frozen files

Focused boundary RED before implementation: `boundary-red.log`, 3 methods, 1 behavioral failure and 2 missing-helper errors. The separate real-writer/full-original-parser RED above is the primary mechanism proof.

Final command:

```sh
python3 -m unittest tests.test_style_tone_response tests.validation.test_workflow_configuration -v
python3 scripts/apple/check_style_tone_response.py --self-test
bash -n scripts/apple/run_style_tone_sweep.sh
git diff --check
```

37 PASS: 18 Tone + 19 workflow tests (`boundary-green.log`); self-test, shell syntax and diff check pass. New tests cover v0/v1 counts, incomplete headers/counts/entries, unknown version, unexpected source hash, persisted failure before execution, actual child-process extraction routing and unchanged upstream source. Prior missing/unique-role/data/finite/metadata/comparison and process-failure regressions remain passing.

Frozen SHA256:

- `scripts/apple/check_style_tone_response.py`: `c485c59b6a5bcb46cc7a0dada87621cde9930103f14d728bf94ddc4761cd5dd3`.
- `tests/test_style_tone_response.py`: `368e6ebecbb7e5fdd3ebac73345cec9f531542f1a01ceaaa5cb50085b4b401e9`.

Exact two-file diff is archived as `boundary.patch`. No staging, commits, push, dispatch, upstream mutation, or media upload by this agent. Current concurrent product work belongs to other owners and was not modified.

## Remaining acceptance

Run the existing Tone job on the new committed head, keeping the strict missing-data gate. Require actual product native textures and all measured fields for every requested photo/Tone before accepting the measurements. If a later native precondition fails, retain that first stderr and continue from it. This parser repair does not establish native or Photos Tone/Color success on its own, and no change to plist version 15 or capture facts is part of it.
