# Direct PVT native-import probe — audit prototype

No repository files changed. No workflow dispatch, push, runtime installation or media upload. This is a four-file prototype for parent review: `native_probe.py`, `GenerateSyntheticMotion.swift`, `PVTReadback.swift`, and this description. The Swift sources have **not been compiled or executed** in this Linux environment.

## Exact route

1. On the existing free macOS runner, record real `xcodebuild -version`, `xcrun simctl help addmedia`, `xcrun simctl help privacy`, installed iOS runtimes/device types, and simulator SDK path/version. No runtime download or toolchain bootstrap. Select an installed compatible iPhone type; otherwise retain its absence as a blocker.
2. Require the requested exact Git HEAD and a clean checkout. Build the current CLI with locked Cargo, parse its compiler-artifact executable path, reject a different `--cli`, and record SHA-256. The caller supplies the already configured Rust/libheif environment; the probe installs nothing.
3. Native public `AVAssetWriter` and ImageIO generate a 256×256 SDR JPEG and H.264 MP4 containing 60 genuine geometric frames at 30 fps, no audio or fixture input. Frame 30 is the cover. The driver adds explicit synthetic Google MicroVideo XMP with the real appended MP4 length and 1-second cover timestamp. This is an independently generated test source, not claimed camera capture metadata.
4. Run current CLI inspect → `convert --live-photo-output pvt` → validate. Record full source/member hashes, actual asset ID, presentation time, and the XML string-version plist. Source generation or CLI validation failure stops before import.
5. Create one uniquely named simulator and record only the exact UDID returned by that call. Boot it. Compile/install an ad-hoc-signed read-only PhotoKit app targeting the actual simulator SDK and host architecture.
6. Only if actual simctl help documents `grant` and `photos`, use `xcrun simctl privacy <this new UDID> grant photos <this installed unique bundle ID>`. This is the parent's explicitly authorized disposable-simulator setup. It never changes host TCC, a privacy database, a globally selected device, an unrelated app, or a real Photos library. If unsupported or unsuccessful, record the readback blocker without another authorization workaround.
7. Record baseline asset IDs. New simulators may contain stock assets. The only import invocation is `xcrun simctl addmedia <this new UDID> <absolute complete capture.pvt directory>`. There is no member fallback, PhotoKit creation API or second import. If readback setup fails but the verified synthetic package and owned simulator are ready, this one direct import is still attempted and its actual exit/stdout/stderr retained.
8. If import exits zero and readback is available, fetch the asset-ID delta. Require exactly one added asset, no removed baseline asset, `photoLive`, one original `.photo`, and one `.pairedVideo`. Export those original resources **after import, solely for verification** into the owned app container. Compute SHA-256 for full resource bytes. Use native ImageIO to read MakerApple 17, AVFoundation to read the video content identifier, and AVAssetReader's timed metadata adaptor to read still-image-time value/timescale/epoch/sample values. Check both IDs against the CLI result and check the exact 1-second cover represented by the reported rational time.
9. Copy exported bytes only into an owned readback-validation directory to run the existing CLI validator. These exports are never passed to an import API. Report native admission, identifier/time checks, and byte preservation separately. A native admission with changed original bytes is **not** an overall PASS. On success and failure, shutdown/delete only the exact simulator created by this invocation; retain owned local JSON and logs. Cleanup has its own status and exact exit codes; a shutdown/delete failure prevents an overall PASS. Timeouts retain available stdout/stderr and mark the command outcome uncertain.

## Native hook invocation

Use the already configured clean runner checkout and current native CLI target path:

```sh
python3 /path/to/probe/native_probe.py \
  --repo "$GITHUB_WORKSPACE" \
  --cli "$GITHUB_WORKSPACE/target/debug/xdremux" \
  --expected-head 0c5789ac3891f5bbfaad629d2500abd417231ff7 \
  --output-root "$RUNNER_TEMP/pvt-native-import" \
  --execute-owned-simulator
```

Omit the execute flag for capability discovery only. This prototype is **not wired into a workflow**. Upload only explicitly selected JSON/text receipts if the parent later adds a hook; do not upload the media, generated app or simulator container. Native output is under a unique `pvt-native-*` work directory. Every command's exact argv, exit code and original stdout/stderr are retained. A missing simulator/runtime/SDK, unsupported direct PVT import, compiler failure, permission failure and readback mismatch are distinct non-PASS outcomes.

The parent's proposed minimal repository integration is three files under `scripts/apple/pvt_native_import/`: rename the driver to `check_native_import.py` and retain both Swift source names. Its relative source lookup already supports that placement. Add only focused pure boundary tests and one independent job to the existing `research.yml`; reuse that owner's exact-HEAD checkout, configured Rust/libheif environment and reviewed free native runner. Existing user authorization covers this isolated native test; no additional user permission is required. Keep output under `RUNNER_TEMP` so the required source-status check stays clean. The artifact list should name `result.json`, `baseline.json`, `after.json` and selected command stdout/stderr explicitly, with no wildcard that could include media, the generated app, exported resources or the complete work directory. No integration or workflow change is part of this audit.

Input and package hashes are observed again in `finally` before simulator cleanup, including native import rejection, timeout or readback failure. A missing/unreadable/changed source is reported without overwriting the original failure; an otherwise successful native validation cannot become an overall PASS without those preservation observations.

## Current evidence and limits

- Initial observed repo HEAD: `91c3ab45310adb05d5e9e3a3d1c23e1345542b06`; parent later committed the independent Tone fix at `0c5789ac3891f5bbfaad629d2500abd417231ff7`. Supply the exact reviewed execution HEAD rather than relying on this example. Product tests and root native CI are separate work.
- Local host: Linux. Actual attempted `xcrun simctl help addmedia` failed with `FileNotFoundError: [Errno 2] No such file or directory: 'xcrun'`. Initial receipt: `local-results/pvt-native-v64vmupt/result.json`; `local-preflight.json` points to the final current-source receipt. A middle invocation rejected stale expected HEAD after the parent's independent commit. No simulator was created, no synthetic media generated and no import attempted here.
- Python syntax and pure checks cover stock-asset baseline subtraction, zero/two added assets, removed baseline IDs, wrong selected asset, non-Live subtype, mismatched pair ID, missing cover time, exact source-wrapper offset and installed-runtime selection. These are prototype logic checks, **not native evidence**. `pure-prototype-checks.json` records that distinction.
- The `details.value` response lookup was checked against the current `0c5789a` CLI JSON writer (`details: report`) and runtime serde enum (`content = "value"`). The probe will parse its actual current-CLI validation output and fail on a different/missing shape; no prior sample JSON is substituted.
- Official Apple material documents simulator media import generally and PhotoKit subtype/resources/authorization; it does not establish that this installed simctl accepts a whole PVT directory. The native addmedia call must settle that question. Rejection is retained; successful member import is not substituted.
- A simulator PASS would establish direct native import plus the reported PhotoKit/native metadata facts on that exact simulator OS. It would not establish physical-device Photos UI editing, dynamic wallpaper eligibility or visual equivalence.

## Primary references

- [Apple WWDC19: Getting the Most Out of Simulator](https://developer.apple.com/videos/play/wwdc2019/418/) demonstrates photo/video recombination in simulator drag-and-drop; it is not proof of this probe's whole-directory route.
- [Apple WWDC20: Become a Simulator expert](https://developer.apple.com/videos/play/wwdc2020/10647/) and [Xcode 11.4 release notes](https://developer.apple.com/documentation/xcode-release-notes/xcode-11_4-release-notes) document simctl's app-specific privacy controls, including Photos. Installed help and the explicit owned UDID/bundle restriction remain required here.
- [Apple PHAssetResource.assetResources(for:)](https://developer.apple.com/documentation/photos/phassetresource/assetresources(for:)-27o4l) and [PHAssetResourceType](https://developer.apple.com/documentation/photos/phassetresourcetype) describe resource enumeration and the original paired video.
- [Apple photoLive](https://developer.apple.com/documentation/photos/phassetmediasubtype/photolive) identifies the Live Photo subtype.
- [Apple PhotoKit authorization](https://developer.apple.com/documentation/photos/phphotolibrary/requestauthorization(for:handler:)) documents authorization; this prototype uses only the separately authorized, help-confirmed simctl setup on its own new simulator.
- [Apple resource data requests](https://developer.apple.com/documentation/photos/phassetresourcemanager/requestdata(for:options:datareceivedhandler:completionhandler:)) and [AVAssetWriter](https://developer.apple.com/documentation/avfoundation/avassetwriter) describe the readback and synthetic generator primitives.

The official web tool retrieved the documentation pages/snippets. Direct urllib requests for their Markdown bodies received proxy 403; `sources.json` records that access limitation. No repeated curl/bootstrap attempt was made, and no missing SDK or help text was fabricated.
