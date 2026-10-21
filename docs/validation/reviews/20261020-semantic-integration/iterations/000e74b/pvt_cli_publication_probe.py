"""Own audit prototype: execute the real CLI under a one-shot FS scheduler."""
import hashlib
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import tempfile
import time

root = Path('/workspace/XDRemux-integration')
audit = Path('/workspace/xdremux-audit')
proof = Path(tempfile.mkdtemp(prefix='pvt-cli-proof-', dir=audit))
work = proof / 'work'
work.mkdir()
cli = proof / 'xdremux-snapshot'
shutil.copy2(root / 'target/debug/xdremux', cli)
fixture = root / 'fixtures/motion-photo/samsung/heif-ultrahdr-01.heic'
commands, results, active = [], {}, []

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def source_hashes():
    paths = subprocess.check_output(['git', 'ls-files', '-z', '*.rs', '*Cargo.toml', 'Cargo.lock'], cwd=root).decode().split('\0')
    paths += ['crates/xdremux-runtime/src/live_photo_package.rs', 'crates/xdremux-cli/tests/live_photo_pvt.rs']
    return {path: sha(root / path) for path in sorted(set(paths)) if path}

def hashes(package):
    return {entry.name: sha(entry) for entry in sorted(package.iterdir())}

def launch(name, output, pause=False):
    output.parent.mkdir(parents=True, exist_ok=True)
    ready, release = proof / (name + '.ready'), proof / (name + '.release')
    env = os.environ.copy()
    if pause:
        env.update(LD_PRELOAD=str(audit / 'pvt_interpose.so'), PVT_READY=str(ready), PVT_RELEASE=str(release))
    args = [str(cli), '--lang', 'en', 'convert', '--input', str(fixture), '--output', str(output), '--live-photo-output', 'pvt']
    process = subprocess.Popen(args, cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    active.append(process)
    return name, args, process, ready, release

def wait_ready(handle):
    _, _, process, ready, _ = handle
    deadline = time.monotonic() + 20
    while not ready.exists():
        assert process.poll() is None, 'CLI exited before the first owned member creation'
        assert time.monotonic() < deadline, 'scheduler deadline'
        time.sleep(0.001)

def finish(handle):
    name, args, process, _, _ = handle
    stdout, stderr = process.communicate(timeout=30)
    (proof / (name + '.stdout')).write_bytes(stdout)
    (proof / (name + '.stderr')).write_bytes(stderr)
    commands.append({'name': name, 'command': args, 'exit_code': process.returncode})
    return process.returncode, stderr.decode()

def validate(name, package):
    args = [str(cli), 'validate', str(package), '--json']
    result = subprocess.run(args, cwd=root, capture_output=True, timeout=20)
    (proof / (name + '.stdout')).write_bytes(result.stdout)
    (proof / (name + '.stderr')).write_bytes(result.stderr)
    commands.append({'name': name, 'command': args, 'exit_code': result.returncode})
    assert result.returncode == 0 and json.loads(result.stdout)['valid'] is True
    metadata = plistlib.loads((package / 'metadata.plist').read_bytes())
    assert metadata == {'PFVideoComplementMetadataVersionKey': '1'}
    return json.loads(result.stdout)

def recovery(name, handle, error):
    retained = Path(handle[3].read_text()).parent
    assert retained.is_dir() and str(retained) in error
    assert len(list(retained.iterdir())) == 3
    validate(name + '-retained-valid', retained)
    return {'path': str(retained), 'sha256': hashes(retained)}

report = {
    'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root).decode().strip(),
    'source_status': subprocess.check_output(['git', 'status', '--porcelain'], cwd=root).decode(),
    'source_hashes_before': source_hashes(),
    'binary_sha256': sha(cli), 'build_log': str(audit / 'pvt-cli-build.log'),
    'build_log_sha256': sha(audit / 'pvt-cli-build.log'),
    'binding': 'cargo build --locked -p xdremux-cli on dirty worktree; snapshot binary and source hashes, not committed HEAD proof',
    'scheduler_sha256': sha(audit / 'pvt_interpose.so'),
    'fixture_sha256_before': sha(fixture), 'results': results, 'commands': commands,
}
build = json.loads((audit / 'pvt-bound-build.json').read_text())
report['bound_build_manifest'] = str(audit / 'pvt-bound-build.json')
report['bound_build_source_and_binary_match'] = (build['exit_code'] == 0 and build['sources_stable_during_build'] and
    report['source_hashes_before'] == build['source_hashes_after'] and report['binary_sha256'] == build['binary_sha256'])
assert report['bound_build_source_and_binary_match']
try:
    output = work / 'same/capture.pvt'
    a = launch('same-a', output, pause=True)
    wait_ready(a)
    b = launch('same-b', output)
    assert finish(b)[0] == 0
    validate('same-winner-valid', output)
    winner_hashes = hashes(output)
    a[-1].write_text('continue')
    code, error = finish(a)
    assert code == 1 and hashes(output) == winner_hashes
    results['cooperative_same_destination'] = {'loser_exit': code, 'winner_preserved': True, 'final_sha256': winner_hashes, 'retained': recovery('same', a, error)}

    alpha, beta = work / 'different/capture.pvt', work / 'different/other.pvt'
    a = launch('different-a', alpha, pause=True)
    wait_ready(a)
    b = launch('different-b', beta)
    assert finish(b)[0] == 0
    b_hashes = hashes(beta)
    a[-1].write_text('continue')
    assert finish(a)[0] == 0 and hashes(beta) == b_hashes
    validate('different-a-valid', alpha)
    validate('different-b-valid', beta)
    results['different_destinations'] = {'both_succeeded': True, 'other_preserved': True}

    for shape in ['file', 'empty-directory', 'dangling-symlink']:
        output = work / shape / 'capture.pvt'
        a = launch(shape, output, pause=True)
        wait_ready(a)
        if shape == 'file':
            output.write_bytes(b'UNOWNED OUTPUT BYTES')
        elif shape == 'empty-directory':
            output.mkdir()
        else:
            output.symlink_to(output.parent / 'absent')
        a[-1].write_text('continue')
        code, error = finish(a)
        assert code == 1
        if shape == 'file':
            assert output.read_bytes() == b'UNOWNED OUTPUT BYTES'
        elif shape == 'empty-directory':
            assert output.is_dir() and list(output.iterdir()) == []
        else:
            assert output.is_symlink() and not output.exists()
        results['late_' + shape] = {'exit': code, 'competitor_preserved': True, 'retained': recovery(shape, a, error)}
finally:
    for process in active:
        if process.poll() is None:
            process.kill()
            process.communicate()
    report['fixture_sha256_after'] = sha(fixture)
    report['binary_sha256_after'] = sha(cli)
    report['source_hashes_after'] = source_hashes()
    report['source_bytes_unchanged'] = report['fixture_sha256_after'] == report['fixture_sha256_before']
    report['product_sources_unchanged_during_probe'] = report['source_hashes_before'] == report['source_hashes_after']
    (proof / 'manifest.json').write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({'proof': str(proof), 'results': list(results), 'source_bytes_unchanged': report['source_bytes_unchanged']}, ensure_ascii=False))
