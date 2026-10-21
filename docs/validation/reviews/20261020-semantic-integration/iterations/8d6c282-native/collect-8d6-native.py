from pathlib import Path
import hashlib
import json
import re
import subprocess
import math
from collections import Counter
from datetime import datetime, timezone

ROOT = Path('/workspace/xdremux-audit')
HEAD = '8d6c2823f015f06ca28d35ef80e4b3e7e2a4971a'
path = ROOT / '8d6c282-system-job119082111552.log'
lines = path.read_text(encoding='utf-8-sig').splitlines()
rows = []
for line in lines:
    text = re.sub(r'^\S+Z ', '', line)
    try:
        row = json.loads(text)
    except ValueError:
        continue
    if isinstance(row, dict) and row.get('photo') == 'lhdr-v1-01.system' and 'components' in row:
        rows.append(row)
assert len(rows) == 3 and {r['tone'] for r in rows} == {-1.0, 0.0, 1.0}
assert all(r['returncode'] == 0 and r['components'] > 0 for r in rows)
assert any('1 passed; 0 failed; 0 ignored;' in line for line in lines)
result = {
    'source': HEAD, 'run': 39557962421, 'job': 119082111552,
    'conclusion': 'success', 'native_rows': rows,
    'synthetic_hdr_roundtrip': '1 passed; 0 failed; 0 ignored;',
    'log_sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
    'limits': ['Actual System HEIF render data only; full Tone, PVT Photos admission/readback, physical device and wallpaper are separate.'],
}
(ROOT / '8d6c282-system-observation.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result, indent=2))

path = ROOT / '8d6c282-product-job119082110843.log'
text = path.read_text(encoding='utf-8-sig')
assert HEAD in text and 'XDRemuxAppModelTests passed' in text
assert 'test apple_styles::tests::style_scene_preserves_every_rgb8_code_and_row_stride ... ok' in text
source = subprocess.check_output(['git', 'show', HEAD + ':apps/macos/XDRemuxApp/Tests/XDRemuxAppModelTests.swift'], cwd='/workspace/XDRemux-integration')
main = source.decode().split('static func main() async throws {', 1)[1].split('print("XDRemuxAppModelTests passed")', 1)[0]
calls = re.findall(r'try (?:await )?(test\w+)\(\)', main)
assert len(calls) == 30 and len(set(calls)) == 30
matrix = [dict(zip(('case', 'format', 'skip'), row), blocked=True) for row in re.findall(r'PVT package boundary: case=([^,]+), format=([^,]+), skip=([^,]+),.*?plan=existingPackage, status=failed, canStart=false', text)]
assert len(matrix) == 32 and len({(r['case'], r['format'], r['skip']) for r in matrix}) == 32
previous = json.loads((ROOT / '392009a-app-observation.json').read_text())
assert previous['registered_model_calls'] == calls
assert previous['native_pvt_matrix'] == matrix
assert previous['model_harness_source_sha256'] == hashlib.sha256(source).hexdigest()
result = {**previous, 'source': HEAD, 'run': 39557962447, 'job': 119082110843,
          'url': 'https://github.com/21Z121Z1/XDRemux/actions/runs/39557962447/job/119082110843',
          'registered_model_calls': calls, 'native_pvt_matrix': matrix,
          'log_sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
          'captured_at': datetime.now(timezone.utc).isoformat(),
          'limits': ['Actual8d6native product/App30/32case evidence only; whole PVT installation/UI/Photos admission/readback is separate.']}
(ROOT / '8d6c282-app-observation.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({'app': result['source'], 'calls': len(calls), 'matrix': len(matrix), 'log_sha256': result['log_sha256']}))

path = ROOT / '8d6c282-package-job119082111180.log'
text = path.read_text(encoding='utf-8-sig')
assert HEAD in text and 'python3 scripts/distribution/smoke_macos.py' in text
result = {
    'source': HEAD, 'run': 39557962429, 'job': 119082111180,
    'conclusion': 'success', 'architecture': 'arm64', 'minimum_product_macos': '27',
    'executed_contract': 'Exact-head pinned dependency/build/archive and extracted package outside checkout with clean system-only process environment; real conversions and output validation required by smoke_macos.py.',
    'log_sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
    'limits': ['The full relocated JSON/artifact was not independently materialized locally; this observation is based on the successful named native job steps, retained console log and inspected exact-head smoke contract.', 'Native feature acceptance, whole-PVT Photos admission and wallpaper are separate.'],
}
(ROOT / '8d6c282-package-observation.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({'package': HEAD, 'log_sha256': result['log_sha256']}))

path = ROOT / '8d6c282-tone-job119082111721.log'
text = path.read_text(encoding='utf-8-sig')
assert HEAD in text
lines = [re.sub(r'^\S+Z ', '', line) for line in text.splitlines()]
rows = []
for line in lines:
    try:
        row = json.loads(line)
    except ValueError:
        continue
    if isinstance(row, dict) and 'photo' in row and 'components' in row:
        rows.append(row)
counts = Counter(r['photo'] for r in rows)
assert len(rows) == 88 and len(counts) == 8 and set(counts.values()) == {11}
assert all(r['returncode'] == 0 and r['components'] > 0 for r in rows)
assert sum(r['photo'].endswith('.xdremux') for r in rows) == 33
assert all(r['photo'].endswith('.xdremux') for r in rows[-33:])
metrics = [line for line in lines if line.startswith('Monotonic mean luminance:')]
assert len(metrics) == 8
for line in metrics[-3:]:
    assert 'Monotonic mean luminance: True.' in line and 'None' not in line
    assert all(math.isfinite(float(v)) for v in re.findall(r'= (-?\d+(?:\.\d+)?)', line))
result = {'source': HEAD, 'run': 39557962421, 'job': 119082111721,
          'conclusion': 'success', 'native_rows': rows, 'all_metrics': metrics,
          'product_metrics': metrics[-3:], 'log_sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
          'limits': ['Actual render data only; whole PVT/physical Photos/wallpaper acceptance separate.']}
(ROOT / '8d6c282-tone-observation.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({'tone': HEAD, 'successful_nonempty_rows': len(rows), 'photos': dict(counts), 'product_metrics': metrics[-3:]}))

path = ROOT / '8d6c282-exact-job119082112604.log'
text = path.read_text(encoding='utf-8-sig')
checks = [{'kind': k, 'name': n, 'duration_seconds': float(t), 'passed': True}
          for k, n, t in re.findall(r'PASS \[(\w+)\] ([\w-]+) \(([\d.]+)s\)', text)]
assert len(checks) == len({c['name'] for c in checks}) == 14
assert 'PASS completion gate for ' + HEAD in text and 'PASS verified completion receipt for ' + HEAD in text
result = {'source': HEAD, 'run': 39557962806, 'job': 119082112604,
          'conclusion': 'success', 'checks': checks,
          'exact_head_receipt_verified_by_ci': True,
          'log_sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
          'artifact': {'id': 12326814848, 'bytes': 17232, 'reported_sha256': 'efb566259c1dd61845a02cd86713d4811fafef8dab4a84845456f87b3252b613', 'local_bytes_verified': False},
          'limits': ['14 actual native completion subchecks and receipt-verifier success are observed in the console. The full receipt artifact is not independently materialized locally.', 'Separate Photos whole-PVT gate remains failed; exact-head success does not override it.']}
(ROOT / '8d6c282-exact-observation.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({'exact': HEAD, 'checks_passed': len(checks)}))

path = ROOT / '8d6c282-performance-job119082111543.log'
text = path.read_text(encoding='utf-8-sig')
text = re.sub(r'\x1b\[[0-9;]*m', '', text)
text = '\n'.join(re.sub(r'^\S+Z ?', '', line) for line in text.splitlines())
objects = []
for match in re.finditer(r'\{', text):
    try:
        value, _ = json.JSONDecoder().raw_decode(text[match.start():])
    except ValueError:
        continue
    if isinstance(value, dict):
        objects.append(value)
arms = next(o for o in objects if 'arms' in o)
comparison = next(o for o in objects if o.get('distinct_revisions') and 'summary' in o)
assert arms['arms']['candidate']['sha'] == HEAD
assert arms['arms']['baseline']['sha'] == '89d806c8aec961a7991eb3e6d6e2b1370aed8791'
assert all(not a['tracked_status'] for a in arms['arms'].values())
samples = [o['sample'] for o in objects if 'sample' in o]
assert len(samples) == 12 and sum(not s['warmup'] for s in samples) == 10
assert 'test apple_styles::tests::style_scene_preserves_every_rgb8_code_and_row_stride ... ok' in text
assert '1 passed; 0 failed; 0 ignored;' in text
budget = [line for line in text.splitlines() if re.match(r'  apple-styles (median_wall_seconds|p95_wall_seconds|max_peak_rss_mib): observed=', line)]
assert len(budget) == 3 and 'performance regression gate passed' in text
result = {'source': HEAD, 'run': 39557962521, 'job': 119082111543, 'attempt': 1, 'conclusion': 'success',
          'native_release_actual_consumer': '1passed0failed0ignored; real consumer test name present and workflow exact guard passed',
          'absolute_styles_budget_lines': budget, 'interleaved_result': comparison, 'arms': arms, 'samples': samples,
          'log_sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
          'limits': ['Absolute gate passed without threshold changes. Same-runner candidate remains slower; no full relative-regression attribution or complete ARM64/E2E speedup claim.', 'All preceding failures/retry reasons remain in inherited evidence. This head had one performance attempt and no rerun.']}
(ROOT / '8d6c282-performance-observation.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({'performance': HEAD, 'budget': budget, 'summary': comparison['summary']}))
