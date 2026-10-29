#!/usr/bin/env python3
"""Select CI work from the same changed-path calculation as completion receipts."""

from __future__ import annotations

import argparse
import fnmatch

from scripts.ci.agent_completion_gate import changed_files, repository_root, resolve_commit


# Changes to selection or its owning workflow exercise every selected consumer.
COMMON = (
    ".github/workflows/research.yml", "scripts/ci/**",
    "tests/validation/test_ci_selection.py",
)
RUST_BUILD = ("Cargo.toml", "Cargo.lock", "rust-toolchain*", ".cargo/**")
SWIFT_BUILD = ("Package.swift", "Package.resolved")
STYLE_PRODUCT = RUST_BUILD + SWIFT_BUILD + (
    "crates/xdremux-cli/**", "crates/xdremux-engine/**",
    "crates/xdremux-runtime/src/apple*.rs",
    "crates/xdremux-runtime/src/style*.rs", "crates/xdremux-runtime/src/lib.rs",
    "crates/xdremux-runtime/tests/apple*.rs",
    "crates/xdremux-heif/**", "crates/xdremux-codec/**", "crates/xdremux-hdr/**",
    "crates/xdremux-format/**", "crates/xdremux-metadata/**",
    "crates/xdremux-source/**", "crates/xdremux-container/**",
    "Sources/**", "fixtures/proxdr/**", "fixtures/SHA256SUMS",
)
RESEARCH = {
    "portable-evidence": RUST_BUILD + SWIFT_BUILD + (
        "tests/**", "research/**", "scripts/**", "xdremux_py/**",
        "model_adapters/**", "crates/**", "Sources/**", "apps/**",
        "pyproject.toml", ".gitignore",
    ),
    "video-carrier": ("research/video_styles/**", "tests/test_video_style*.py"),
    "coreml-candidate": (
        "research/oppo_styles/CoreMLProbe.swift", "research/oppo_styles/models/**",
        "research/texture_styles/oracle.py", "research/texture_styles/sparse.py",
        "tests/test_texture_sparse_lowering.py", "tests/test_texture_evidence_provenance.py",
    ),
    "texture-probe-build": (
        "research/texture_styles/**", "tests/test_texture_research_probe.py",
    ),
    "styles-version-control": STYLE_PRODUCT + (
        "scripts/apple/check_styles_version_hypothesis.sh",
        "research/oppo_styles/tools/probe_style_version.py", "tests/test_style_version_probe.py",
    ),
    "style-tone-response": STYLE_PRODUCT + (
        "scripts/apple/run_style_tone_sweep.sh", "scripts/apple/check_style_tone_response.py",
        "scripts/apple/check_rust_style_consumer.sh", "tests/test_style_tone_response.py",
        "research/oppo_styles/tools/measure_style_editor_response.py",
    ),
    "system-heif-hdr-api": STYLE_PRODUCT + (
        "scripts/apple/probe_system_heif_hdr_api.sh", "scripts/apple/measure_system_heif_hdr.sh",
        "scripts/apple/score_system_hdr_tone.py", "tests/test_style_tone_response.py",
        "crates/xdremux-runtime/tests/system_heif_hdr.rs",
    ),
    "pvt-native-import": RUST_BUILD + (
        "scripts/apple/pvt_native_import/**", "tests/test_pvt*.py",
        "tests/fixtures/pvt_boot_status/**", "crates/xdremux-motion-photo/**",
        "crates/xdremux-runtime/src/live_photo/**", "crates/xdremux-runtime/src/lib.rs",
        "crates/xdremux-cli/**", "crates/xdremux-engine/**",
        "crates/xdremux-source/**", "crates/xdremux-container/**",
        "crates/xdremux-format/**", "crates/xdremux-heif/**",
        "crates/xdremux-codec/**", "crates/xdremux-hdr/**",
        "crates/xdremux-metadata/**", "fixtures/**",
    ),
}


def select(paths: list[str], *, manual: bool = False) -> dict[str, str]:
    code_paths = [path for path in paths if not path.endswith(".md")]
    product = manual or bool(code_paths)
    result = {
        "runner": "xcode-27" if product else "ubuntu-latest",
        "plan": "product" if product else "documentation",
        "product": str(product).lower(),
    }
    for job, patterns in RESEARCH.items():
        selected = manual or any(
            fnmatch.fnmatchcase(path, pattern)
            for path in code_paths for pattern in COMMON + patterns
        )
        result[job] = str(selected).lower()
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", required=True)
    parser.add_argument("--head", required=True)
    parser.add_argument("--event", choices=("pull_request", "push", "workflow_dispatch"), required=True)
    args = parser.parse_args()
    repo = repository_root()
    head = resolve_commit(repo, args.head)
    if head != resolve_commit(repo, "HEAD"):
        parser.error("the checkout does not match the requested head")
    if not args.base and args.event != "workflow_dispatch":
        parser.error("automatic runs require the event base commit")
    base = resolve_commit(repo, args.base or "HEAD^")
    paths = changed_files(repo, base, head)
    print("base=" + base)
    for name, value in select(paths, manual=args.event == "workflow_dispatch").items():
        print(name + "=" + value)


if __name__ == "__main__":
    main()
