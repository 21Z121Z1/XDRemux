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
cli = root / 'target/debug/xdremux'
fixture = root / 'fixtures/motion-photo/samsung/heif-ultrahdr-01.heic'
shim = audit / 'lifecycle_interpose.so'
proof = Path(tempfile.mkdtemp(prefix='lifecycle-proof-', dir=audit))
work = Path(tempfile.mkdtemp(prefix='lifecycle-work-', dir=audit))
commands = []
results = {}

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def record(name, args, completed):
    (proof / f'{name}.stdout').write_bytes(completed.stdout)
    (proof / f'{name}.stderr').write_bytes(completed.stderr)
    commands.append({'name': name, 'command': [str(a) for a in args],
                     'exit_code': completed.returncode})
    return completed

def run(name, args, env=None):
    args = [str(a) for a in args]
    return record(name, args, subprocess.run(args, cwd=root, env=env,
                                            capture_output=True, timeout=30))

def convert(name, image, source=fixture, env=None):
    return run(name, [cli, 'convert', '--input', source, '--output', image], env)

def injected(mode, image, **extra):
    env = os.environ.copy()
    env.update(LD_PRELOAD=str(shim), XDREMUX_AUDIT_MODE=mode,
               XDREMUX_AUDIT_IMAGE=str(image),
               XDREMUX_AUDIT_MOVIE=str(image.with_suffix('.mov')))
    env.update({f'XDREMUX_AUDIT_{key}': str(value) for key, value in extra.items()})
    return env

def validate(name, image):
    result = run(name, [cli, 'validate', image, '--json'])
    assert result.returncode == 0, result.stderr.decode()
    assert json.loads(result.stdout)['valid'] is True

def paused_pair(name, image, outer_image, mode):
    ready = proof / f'{name}.ready'
    release = proof / f'{name}.release'
    args = [str(cli), 'convert', '--input', str(fixture), '--output', str(image)]
    process = subprocess.Popen(args, cwd=root, env=injected(mode, image, READY=ready, RELEASE=release),
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    try:
        deadline = time.monotonic() + 15
        while not ready.exists():
            assert process.poll() is None, 'process ended before scheduling point'
            assert time.monotonic() < deadline, 'pause hook was not reached'
            time.sleep(0.001)
        before = sorted(str(p.relative_to(work)) for p in work.rglob('*'))
        other = convert(name + '-other', outer_image)
        assert other.returncode == 0, other.stderr.decode()
        after_other = sorted(str(p.relative_to(work)) for p in work.rglob('*'))
        release.write_text('continue\n')
        stdout, stderr = process.communicate(timeout=15)
        record(name, args, subprocess.CompletedProcess(args, process.returncode, stdout, stderr))
        return {'exit_code': process.returncode, 'stderr': stderr.decode(),
                'before_other': before, 'after_other': after_other,
                'image_exists': image.exists(), 'movie_exists': image.with_suffix('.mov').exists()}
    finally:
        if process.poll() is None:
            process.kill()
            process.communicate()

report = {'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root).decode().strip(),
          'binary': str(cli), 'binary_sha256': digest(cli),
          'build_binding': 'existing binary; build-to-current-HEAD binding not established',
          'fixture_sha256_before': digest(fixture), 'results': results}
try:
    assert report['fixture_sha256_before'] == '06eb244bc69ae464bd7b0a60b769f4fc3429dc543451481f5331586a7536b8d0'
    assert run('help', [cli, '--help']).returncode == 0
    inspection = run('inspect', [cli, 'inspect', fixture, '--json'])
    assert inspection.returncode == 0
    assert json.loads(inspection.stdout)['kind'] == 'motion-photo'

    existing = work / 'existing/capture.heic'
    existing.parent.mkdir()
    existing.write_bytes(b'EXISTING STILL\n')
    existing.with_suffix('.mov').write_bytes(b'EXISTING MOVIE\n')
    result = convert('existing', existing)
    results['existing_at_entry'] = {'exit_code': result.returncode,
        'image_preserved': existing.read_bytes() == b'EXISTING STILL\n',
        'movie_preserved': existing.with_suffix('.mov').read_bytes() == b'EXISTING MOVIE\n'}
    assert result.returncode == 1 and all(value is True for key, value in results['existing_at_entry'].items() if key.endswith('preserved'))

    late = work / 'late/capture.heic'
    result = convert('late', late, env=injected('late-competitor', late))
    results['late_competitor'] = {'exit_code': result.returncode,
        'image_preserved': late.read_bytes() == b'COMPETITOR STILL\n',
        'movie_preserved': late.with_suffix('.mov').read_bytes() == b'COMPETITOR MOVIE\n',
        'hook_fired': b'competitor files installed' in result.stderr}
    assert result.returncode == 0 and results['late_competitor']['hook_fired']
    validate('late-valid', late)

    destructive = work / 'source-directory'
    victim = destructive / '.capture.unowned.tmp.heic'
    victim.mkdir(parents=True)
    copied_source = victim / 'source.heic'
    shutil.copyfile(fixture, copied_source)
    output = destructive / 'capture.heic'
    result = convert('source-directory', output, source=copied_source)
    results['source_directory'] = {'exit_code': result.returncode,
        'source_exists': copied_source.exists(), 'source_parent_exists': victim.exists(),
        'image_exists': output.exists(), 'movie_exists': output.with_suffix('.mov').exists()}
    assert result.returncode == 0 and not copied_source.exists()
    validate('source-directory-valid', output)

    siblings = work / 'siblings'
    first = siblings / 'capture.variant.heic'
    outer = siblings / 'capture.heic'
    result = paused_pair('active-prefix', first, outer, 'pause-before-movie')
    results['active_prefix'] = result
    assert result['exit_code'] == 1 and 'temporary pair is incomplete' in result['stderr']
    validate('active-prefix-other-valid', outer)

    nested = work / 'nested'
    first = nested / '.capture.unowned.tmp.heic' / 'nested.heic'
    outer = nested / 'capture.heic'
    result = paused_pair('removed-parent', first, outer, 'pause-before-image')
    results['removed_parent'] = result
    assert result['exit_code'] == 1 and 'temporary create' in result['stderr'] and 'No such file or directory' in result['stderr']
    validate('removed-parent-other-valid', outer)

    failed = work / 'rollback/capture.heic'
    result = convert('rollback', failed, env=injected('competitor-before-failed-movie', failed,
                     SEED_IMAGE=late, SEED_MOVIE=late.with_suffix('.mov')))
    results['rollback_competitor'] = {'exit_code': result.returncode,
        'image_exists': failed.exists(), 'movie_exists': failed.with_suffix('.mov').exists(),
        'remaining_movie_is_competitor': failed.with_suffix('.mov').read_bytes() == late.with_suffix('.mov').read_bytes(),
        'hook_fired': b'valid competitor pair installed' in result.stderr}
    assert result.returncode == 1 and results['rollback_competitor']['hook_fired']
    assert not failed.exists() and results['rollback_competitor']['remaining_movie_is_competitor']
finally:
    report['fixture_sha256_after'] = digest(fixture)
    report['binary_sha256_after'] = digest(cli)
    report['commands'] = commands
    shutil.rmtree(work)
    report['scratch_removed'] = not work.exists()
    (proof / 'manifest.json').write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({'proof': str(proof), 'results': results}, ensure_ascii=False, indent=2))
