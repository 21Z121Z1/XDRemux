#!/usr/bin/env bash
# Record the macOS SDK and runtime names for a system HEIF HDR / gain-map encode.
# This script prints what the headers and compiler accept. It does not choose an API.
set -euo pipefail

out="${1:?output directory is required}"
mkdir -p "$out"

{
  sw_vers
  uname -a
  xcodebuild -version
  xcrun --sdk macosx --show-sdk-path
  xcrun --sdk macosx --show-sdk-version
  xcrun --sdk macosx --show-sdk-build-version
} > "$out/host.txt" 2>&1

sdk="$(xcrun --sdk macosx --show-sdk-path)"
printf '%s\n' "$sdk" > "$out/sdk-path.txt"

find "$sdk/System/Library/Frameworks/CoreImage.framework" \
  "$sdk/System/Library/Frameworks/ImageIO.framework" \
  "$sdk/System/Library/Frameworks/CoreGraphics.framework" \
  -name '*.h' -type f > "$out/headers.txt"

python3 - "$sdk" "$out/header-matches.txt" <<'PY'
import pathlib, re, sys
sdk = pathlib.Path(sys.argv[1])
needles = re.compile(
    r"HEIF|HDR|GainMap|gain map|Headroom|headroom|21496|Representation",
    re.IGNORECASE,
)
roots = [
    sdk / "System/Library/Frameworks/CoreImage.framework",
    sdk / "System/Library/Frameworks/ImageIO.framework",
    sdk / "System/Library/Frameworks/CoreGraphics.framework",
]
rows = []
for root in roots:
    for path in root.rglob("*.h"):
        text = path.read_text(encoding="utf-8", errors="replace").splitlines()
        for number, line in enumerate(text, 1):
            if needles.search(line):
                start = max(1, number - 6)
                end = min(len(text), number + 10)
                rows.append(f"----- {path}:{number} -----")
                for cursor in range(start, end + 1):
                    rows.append(f"{path}:{cursor}:{text[cursor - 1]}")
pathlib.Path(sys.argv[2]).write_text("\n".join(rows) + "\n", encoding="utf-8")
print(f"matched {len(rows)} header lines")
PY

# Compile each discovered CFString / Swift constant name on its own so a missing
# symbol does not hide the others. Names come only from the header text.
python3 - "$out" <<'PY'
import pathlib, re, subprocess, sys
out = pathlib.Path(sys.argv[1])
text = (out / "header-matches.txt").read_text(encoding="utf-8")
names = sorted(set(re.findall(r"\bk[A-Z][A-Za-z0-9_]+\b", text)))
interesting = [
    name for name in names
    if re.search(r"HEIF|HDR|Gain|Headroom|21496|Representation", name)
]
(out / "candidate-symbols.txt").write_text("\n".join(interesting) + "\n", encoding="utf-8")
work = out / "compile"
work.mkdir(exist_ok=True)
results = []
for name in interesting:
    source = work / f"{name}.swift"
    source.write_text(
        "import CoreImage\nimport ImageIO\nimport CoreGraphics\n"
        f"@main struct Probe {{\n  static func main() {{\n    print({name})\n  }}\n}}\n",
        encoding="utf-8",
    )
    binary = work / name
    completed = subprocess.run(
        ["swiftc", "-parse-as-library", "-O", "-o", str(binary), str(source)],
        text=True,
        capture_output=True,
    )
    status = "compile-ok" if completed.returncode == 0 else "compile-failed"
    detail = (completed.stderr or completed.stdout).strip().splitlines()
    results.append(f"{name}\t{status}\t{detail[-1] if detail else ''}")
    if completed.returncode == 0:
        run = subprocess.run([str(binary)], text=True, capture_output=True)
        (work / f"{name}.stdout").write_text(run.stdout + run.stderr, encoding="utf-8")
(out / "symbol-compile.txt").write_text("\n".join(results) + "\n", encoding="utf-8")
print(f"compiled {len(interesting)} candidate symbols")
PY

# Method names are not constants. Compile the two representation calls the
# headers actually declare, recording the full compiler diagnostic either way.
python3 - "$out/header-matches.txt" "$out/representation-methods.txt" <<'PY'
import pathlib, re, sys
text = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
methods = sorted(set(re.findall(r"\b(?:write|heif)[A-Za-z0-9_]*Representation[A-Za-z0-9_]*\b", text)))
pathlib.Path(sys.argv[2]).write_text("\n".join(methods) + "\n", encoding="utf-8")
print("methods", ", ".join(methods))
PY
