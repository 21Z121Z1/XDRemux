#!/usr/bin/env python3
from pathlib import Path


def replace_once(path: str, old: str, new: str, marker: str) -> None:
    target = Path(path)
    text = target.read_text()
    if marker in text:
        return
    if old not in text:
        raise SystemExit(f"anchor not found in {path}: {old!r}")
    target.write_text(text.replace(old, new, 1))


replace_once(
    "crates/xdremux-runtime/src/lib.rs",
    "mod batch;\nmod live_photo;\n",
    "mod batch;\nmod categorize;\nmod live_photo;\n",
    "mod categorize;",
)
replace_once(
    "crates/xdremux-runtime/src/lib.rs",
    "pub use batch::{BatchAssetKind, BatchFailure, BatchItem, BatchReceipt, BatchSuccess};\npub use live_photo::LivePhotoFileReceipt;\n",
    "pub use batch::{BatchAssetKind, BatchFailure, BatchItem, BatchReceipt, BatchSuccess};\npub use categorize::{CategorizeDisposition, CategorizeItemReceipt, CategorizeReceipt};\npub use live_photo::LivePhotoFileReceipt;\n",
    "pub use categorize::",
)

replace_once(
    "crates/xdremux-cli/src/lib.rs",
    "#![forbid(unsafe_code)]\n\n",
    "#![forbid(unsafe_code)]\n\nmod categorize;\n\n",
    "mod categorize;",
)
replace_once(
    "crates/xdremux-cli/src/lib.rs",
    "    /// Convert a deterministic batch of supported assets.\n    Batch(BatchArgs),\n",
    "    /// Convert a deterministic batch of supported assets.\n    Batch(BatchArgs),\n    /// Classify photo assets and publish them into deterministic folders.\n    Categorize(categorize::CategorizeArgs),\n",
    "Categorize(categorize::CategorizeArgs)",
)
replace_once(
    "crates/xdremux-cli/src/lib.rs",
    "        Ok(Cli {\n            command: RootCommand::Batch(arguments),\n        }) => run_batch(arguments, stdout, stderr),\n        Err(error) => write_clap_error(error, stdout, stderr),\n",
    "        Ok(Cli {\n            command: RootCommand::Batch(arguments),\n        }) => run_batch(arguments, stdout, stderr),\n        Ok(Cli {\n            command: RootCommand::Categorize(arguments),\n        }) => categorize::run(arguments, stdout, stderr),\n        Err(error) => write_clap_error(error, stdout, stderr),\n",
    "command: RootCommand::Categorize(arguments)",
)
replace_once(
    "crates/xdremux-cli/src/lib.rs",
    "        assert!(output.contains(\"batch\"));\n        assert!(stderr.is_empty());\n",
    "        assert!(output.contains(\"batch\"));\n        assert!(output.contains(\"categorize\"));\n        assert!(stderr.is_empty());\n",
    "assert!(output.contains(\"categorize\"));",
)

categorize = Path("crates/xdremux-runtime/src/categorize.rs")
text = categorize.read_text()
text = text.replace("if prefix[1] == b'p' {", "if prefix.len() == 6 {")
categorize.write_text(text)
