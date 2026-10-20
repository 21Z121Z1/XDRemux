import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time

root = Path('/workspace/XDRemux-integration')
audit = Path('/workspace/xdremux-audit')
proof = Path(tempfile.mkdtemp(prefix='lifecycle-competition-proof-', dir=audit))
work = Path(tempfile.mkdtemp(prefix='lifecycle-competition-work-', dir=audit))
cli_source = root / 'target/debug/xdremux'
cli = work / 'xdremux-snapshot'
shutil.copyfile(cli_source, cli)
cli.chmod(0o700)
fixture = root / 'fixtures/motion-photo/samsung/heif-ultrahdr-01.heic'
commands = []
results = {}
active = []

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def hashes(image):
    return [sha(image), sha(image.with_suffix('.mov'))]

def launch(name, image, mode=None):
    ready, release = proof / (name + '.ready'), proof / (name + '.release')
    env = os.environ.copy()
    if mode:
        env.update(LD_PRELOAD=str(audit / 'lifecycle_interpose.so'),
                   XDREMUX_AUDIT_MODE=mode, XDREMUX_AUDIT_IMAGE=str(image),
                   XDREMUX_AUDIT_MOVIE=str(image.with_suffix('.mov')),
                   XDREMUX_AUDIT_READY=str(ready), XDREMUX_AUDIT_RELEASE=str(release))
    args = [str(cli), 'convert', '--input', str(fixture), '--output', str(image)]
    process = subprocess.Popen(args, env=env, cwd=root, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    active.append(process)
    return (name, args, process, ready, release)

def wait_ready(handle):
    _, _, process, ready, _ = handle
    deadline = time.monotonic() + 15
    while not ready.exists():
        assert process.poll() is None, 'conversion exited before scheduling point'
        assert time.monotonic() < deadline
        time.sleep(0.001)

def finish(handle):
    name, args, process, _, _ = handle
    stdout, stderr = process.communicate(timeout=15)
    (proof / (name + '.stdout')).write_bytes(stdout)
    (proof / (name + '.stderr')).write_bytes(stderr)
    commands.append({'name': name, 'command': args, 'exit_code': process.returncode})
    return process.returncode

def validate(name, image):
    args = [str(cli), 'validate', str(image), '--json']
    result = subprocess.run(args, cwd=root, capture_output=True, timeout=15)
    (proof / (name + '.stdout')).write_bytes(result.stdout)
    (proof / (name + '.stderr')).write_bytes(result.stderr)
    commands.append({'name': name, 'command': args, 'exit_code': result.returncode})
    assert result.returncode == 0 and json.loads(result.stdout)['valid'] is True

report = {'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root).decode().strip(),
          'source_status': subprocess.check_output(['git', 'status', '--porcelain'], cwd=root).decode(),
          'binary_sha256': sha(cli), 'fixture_sha256_before': sha(fixture),
          'build_binding': 'copied existing binary; current-HEAD build binding not established',
          'results': results, 'commands': commands}
try:
    image = work / 'same/capture.heic'
    a = launch('same-a', image, 'pause-before-image')
    wait_ready(a)
    b = launch('same-b', image)
    b_code = finish(b)
    assert b_code == 0
    validate('same-b-valid', image)
    b_hashes = hashes(image)
    a[-1].write_text('continue\n')
    a_code = finish(a)
    validate('same-a-valid', image)
    results['same_output'] = {'a_exit': a_code, 'b_exit': b_code,
        'b_pair_hashes': b_hashes, 'final_hashes': hashes(image),
        'b_pair_preserved': b_hashes == hashes(image)}
    assert a_code == 0 and not results['same_output']['b_pair_preserved']

    alpha, beta = work / 'different/alpha.heic', work / 'different/beta.heic'
    a = launch('different-a', alpha, 'pause-before-movie')
    wait_ready(a)
    b = launch('different-b', beta)
    b_code = finish(b)
    b_hashes = hashes(beta)
    a[-1].write_text('continue\n')
    a_code = finish(a)
    validate('different-a-valid', alpha)
    validate('different-b-valid', beta)
    results['different_output'] = {'a_exit': a_code, 'b_exit': b_code,
        'b_pair_preserved': hashes(beta) == b_hashes}
    assert a_code == 0 and b_code == 0 and results['different_output']['b_pair_preserved']

    image = work / 'rollback/capture.heic'
    b = launch('rollback-b', image, 'pause-before-image')
    wait_ready(b)
    a = launch('rollback-a', image, 'pause-fail-movie')
    wait_ready(a)
    b[-1].write_text('continue\n')
    b_code = finish(b)
    assert b_code == 0
    validate('rollback-b-valid-before-a-failure', image)
    b_hashes = hashes(image)
    a[-1].write_text('fail original movie rename\n')
    a_code = finish(a)
    results['failed_rollback_after_competitor'] = {'a_exit': a_code, 'b_exit': b_code,
        'b_valid_before_failure': True, 'b_hashes': b_hashes,
        'image_exists': image.exists(), 'movie_exists': image.with_suffix('.mov').exists(),
        'competitor_movie_preserved': sha(image.with_suffix('.mov')) == b_hashes[1]}
    assert a_code == 1 and not image.exists() and results['failed_rollback_after_competitor']['competitor_movie_preserved']
finally:
    for process in active:
        if process.poll() is None:
            process.kill()
            process.communicate()
    report['fixture_sha256_after'] = sha(fixture)
    report['snapshot_sha256_after'] = sha(cli)
    shutil.rmtree(work)
    report['scratch_removed'] = not work.exists()
    (proof / 'manifest.json').write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({'proof': str(proof), 'results': results}, ensure_ascii=False, indent=2))
