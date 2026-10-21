"""Run portable lifecycle checks here and in an unrelated relocated bundle."""
from pathlib import Path
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tempfile

HERE = Path(__file__).resolve().parent
result = {'host': platform.platform(), 'python': sys.version,
          'native_compilation_executed': False, 'macos_stage_execution': False,
          'stage_cost_result_obtained': False, 'runs': []}
bundle = ['bounded_stage_lifecycle.py', 'e107-stage-attribution-prototype.py',
          'test_e107_stage_prototype.py', 'fixture-provenance.json',
          'fixtures/baseline-apple-styles.rs', 'fixtures/candidate-apple-styles.rs']
result['bundle_sha256'] = {name: hashlib.sha256((HERE / name).read_bytes()).hexdigest()
                           for name in bundle}
def run(source, cwd, name):
    env = {k: v for k, v in os.environ.items()
           if k not in ('PYTHONPATH', 'XDREMUX_STAGE_SOURCE', 'XDREMUX_STAGE_TEST_RECEIPTS')}
    env['PYTHONDONTWRITEBYTECODE'] = '1'
    env['XDREMUX_STAGE_TEST_RECEIPTS'] = str(HERE / 'test-evidence' / name)
    command = [sys.executable, '-m', 'unittest', 'discover', '-s', str(source),
               '-p', 'test_*.py', '-v']
    receipt = subprocess.run(command, cwd=cwd, env=env, capture_output=True, text=True, timeout=20)
    (HERE / (name + '.stdout.log')).write_text(receipt.stdout)
    (HERE / (name + '.stderr.log')).write_text(receipt.stderr)
    match = re.search(r'Ran (\d+) tests', receipt.stderr)
    row = {'name': name, 'command': command, 'cwd': str(cwd), 'exit_code': receipt.returncode,
           'tests': int(match[1]) if match else None, 'stdout': name + '.stdout.log',
           'stderr': name + '.stderr.log', 'relocated_non_git_bundle': name == 'relocated'}
    result['runs'].append(row)
    (HERE / 'portable-verification.json').write_text(json.dumps(result, indent=2) + '\n')
    assert receipt.returncode == 0 and row['tests'] == 22, receipt.stderr

run(HERE, HERE, 'in-place')
with tempfile.TemporaryDirectory(prefix='XDRemux stage moved ') as raw:
    base = Path(raw)
    destination = base / '独立目录 with spaces'
    for name in bundle:
        target = destination / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(HERE / name, target)
    assert not (destination / '.git').exists()
    run(destination, base, 'relocated')
result['passed'] = True
(HERE / 'portable-verification.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result, indent=2))
