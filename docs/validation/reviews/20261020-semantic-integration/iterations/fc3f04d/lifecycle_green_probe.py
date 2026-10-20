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
proof = Path(tempfile.mkdtemp(prefix='lifecycle-green-proof-', dir=audit))
work = Path(tempfile.mkdtemp(prefix='lifecycle-green-work-', dir=audit))
cli = work / 'xdremux-snapshot'
shutil.copyfile(root / 'target/debug/xdremux', cli)
cli.chmod(0o700)
fixture = root / 'fixtures/motion-photo/samsung/heif-ultrahdr-01.heic'
commands, results, active = [], {}, []

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def hashes(image):
    return [sha(image), sha(image.with_suffix('.mov'))]

def invoke(name, args, extra=None):
    env = os.environ.copy()
    env.update(extra or {})
    result = subprocess.run([str(cli), *map(str, args)], cwd=root, env=env,
                            capture_output=True, timeout=20)
    record(name, [str(cli), *map(str, args)], result.returncode, result.stdout, result.stderr)
    return result

def record(name, args, code, stdout, stderr):
    (proof / (name + '.stdout')).write_bytes(stdout)
    (proof / (name + '.stderr')).write_bytes(stderr)
    commands.append({'name': name, 'command': args, 'exit_code': code})

def mode_env(name, image, mode, seed=None):
    env = dict(LD_PRELOAD=str(audit / 'lifecycle_interpose_green.so'),
               XDREMUX_AUDIT_MODE=mode, XDREMUX_AUDIT_IMAGE=str(image),
               XDREMUX_AUDIT_MOVIE=str(image.with_suffix('.mov')),
               XDREMUX_AUDIT_READY=str(proof / (name + '.ready')),
               XDREMUX_AUDIT_RELEASE=str(proof / (name + '.release')))
    if seed:
        env.update(XDREMUX_AUDIT_SEED_IMAGE=str(seed),
                   XDREMUX_AUDIT_SEED_MOVIE=str(seed.with_suffix('.mov')))
    return env

def convert(name, image, source=fixture, mode=None, seed=None):
    env = mode_env(name, image, mode, seed) if mode else None
    return invoke(name, ['convert', '--input', source, '--output', image], env)

def launch(name, image, mode):
    env = os.environ.copy()
    env.update(mode_env(name, image, mode))
    args = [str(cli), 'convert', '--input', str(fixture), '--output', str(image)]
    process = subprocess.Popen(args, env=env, cwd=root, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    active.append(process)
    return (name, args, process, proof / (name + '.ready'), proof / (name + '.release'))

def wait_ready(handle):
    _, _, process, ready, _ = handle
    deadline = time.monotonic() + 15
    while not ready.exists():
        assert process.poll() is None, f'conversion exited before scheduling point: {handle[0]}'
        assert time.monotonic() < deadline, f'scheduling timeout: {handle[0]}'
        time.sleep(0.001)

def finish(handle):
    name, args, process, _, _ = handle
    stdout, stderr = process.communicate(timeout=15)
    record(name, args, process.returncode, stdout, stderr)
    return process.returncode

def validate(name, image):
    result = invoke(name, ['validate', image, '--json'])
    assert result.returncode == 0 and json.loads(result.stdout)['valid'] is True

def staging(image):
    return sorted(p for p in image.parent.glob('.xdremux-live-photo-*') if p.is_dir())

def check_staged(name, image, error, partial_matches=True):
    retained = staging(image)
    assert len(retained) == 1, retained
    capture = retained[0] / 'capture.heic'
    assert capture.is_file() and capture.with_suffix('.mov').is_file()
    validate(name + '-retained-valid', capture)
    if partial_matches:
        assert sha(image.with_suffix('.mov')) == sha(capture.with_suffix('.mov'))
    stderr = error.stderr.decode()
    assert str(retained[0]) in stderr and str(image.with_suffix('.mov')) in stderr
    return {'retained_directory_reported': True, 'complete_staged_pair_valid': True,
            'staged_pair_hashes': hashes(capture), 'partial_movie_matches_staging': partial_matches}

owned_paths = ['crates/xdremux-motion-photo/src/publication.rs',
               'crates/xdremux-runtime/src/live_photo.rs',
               'crates/xdremux-runtime/tests/live_photo_negative.rs',
               'docs/architecture.en.md', 'docs/architecture.md',
               'docs/liblivephoto-integration.en.md', 'docs/liblivephoto-integration.md']
report = {'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root).decode().strip(),
          'source_status': subprocess.check_output(['git', 'status', '--porcelain'], cwd=root).decode(),
          'binary_sha256': sha(cli), 'fixture_sha256_before': sha(fixture),
          'build_binding': 'cargo build --locked -p xdremux-cli against the recorded uncommitted product files; copied snapshot',
          'source_sha256': {p: sha(root / p) for p in owned_paths},
          'results': results, 'commands': commands,
          'fault_scope': 'Linux LD_PRELOAD schedules actual CLI operations; EIO/EOPNOTSUPP are injected, not actual disk/mount failures'}
try:
    image = work / 'entry/capture.heic'
    image.parent.mkdir()
    image.write_bytes(b'EXISTING STILL')
    image.with_suffix('.mov').write_bytes(b'EXISTING MOVIE')
    original = hashes(image)
    result = convert('existing', image)
    assert result.returncode == 1 and hashes(image) == original and not staging(image)
    results['existing_pair'] = {'exit': result.returncode, 'bytes_preserved': True}

    image = work / 'late/capture.heic'
    result = convert('late', image, mode='late-competitor')
    assert result.returncode == 1 and image.read_bytes() == b'COMPETITOR STILL\n'
    assert image.with_suffix('.mov').read_bytes() == b'COMPETITOR MOVIE\n' and not staging(image)
    results['late_competitor'] = {'exit': result.returncode, 'both_entries_preserved': True,
                                 'owned_staging_removed_before_first_claim': True}

    image = work / 'source/capture.heic'
    source = image.parent / '.capture.unowned.tmp.heic/source.heic'
    source.parent.mkdir(parents=True)
    shutil.copyfile(fixture, source)
    source_hash = sha(source)
    result = convert('source-directory', image, source=source)
    assert result.returncode == 0 and source.is_file() and sha(source) == source_hash
    validate('source-directory-valid', image)
    assert not staging(image)
    results['temporary_looking_source_directory'] = {'exit': result.returncode,
                                                    'source_and_parent_preserved': True}

    image = work / 'same/capture.heic'
    a = launch('same-a', image, 'pause-before-image')
    wait_ready(a)
    b = convert('same-b', image)
    assert b.returncode == 0
    validate('same-b-valid', image)
    original = hashes(image)
    a[-1].write_text('continue\n')
    code = finish(a)
    assert code == 1 and original == hashes(image) and not staging(image)
    results['same_output_competition'] = {'a_exit': code, 'b_exit': b.returncode,
                                        'complete_competitor_pair_preserved': True}

    image, alternate = work / 'extension/capture.heic', work / 'extension/capture.heif'
    a = launch('extension-a', image, 'pause-before-image')
    wait_ready(a)
    b = convert('extension-b', alternate)
    assert b.returncode == 0
    validate('extension-b-valid', alternate)
    original = hashes(alternate)
    a[-1].write_text('continue\n')
    code = finish(a)
    assert code == 1 and not image.exists() and hashes(alternate) == original and not staging(image)
    results['shared_mov_different_still_extensions'] = {'a_exit': code, 'b_exit': b.returncode,
                                                      'complete_competitor_pair_preserved': True}

    for label, outer, inner in [
        ('prefix', work / 'prefix/capture.heic', work / 'prefix/capture.variant.heic'),
        ('nested-parent', work / 'nested/capture.heic',
         work / 'nested/.capture.unowned.tmp.heic/beta.heic'),
        ('different', work / 'different/alpha.heic', work / 'different/beta.heic')]:
        b = launch(label + '-b', inner, 'pause-before-movie')
        wait_ready(b)
        still_staging = Path(b[3].read_text()).with_suffix('.heic')
        assert still_staging.is_file()
        a = convert(label + '-a', outer)
        assert a.returncode == 0 and still_staging.is_file()
        b[-1].write_text('continue\n')
        code = finish(b)
        assert code == 0
        validate(label + '-a-valid', outer)
        validate(label + '-b-valid', inner)
        assert not staging(outer) and not staging(inner)
        results[label + '_parallel_output'] = {'a_exit': a.returncode, 'b_exit': code,
                                               'active_staging_and_parent_preserved': True}

    image = work / 'partial/capture.heic'
    result = convert('partial', image, mode='late-image-only')
    assert result.returncode == 1 and image.read_bytes() == b'COMPETITOR STILL\n'
    results['second_member_failure'] = {'exit': result.returncode, 'competitor_still_preserved': True,
                                        **check_staged('partial', image, result)}

    image = work / 'cooperative-partial/capture.heic'
    b = launch('cooperative-b', image, 'pause-before-image')
    wait_ready(b)
    a = launch('cooperative-a', image, 'pause-fail-still-link')
    wait_ready(a)
    assert image.with_suffix('.mov').is_file() and not image.exists()
    b[-1].write_text('continue\n')
    b_code = finish(b)
    assert b_code == 1
    a[-1].write_text('fail second member\n')
    a_code = finish(a)
    assert a_code == 1 and image.with_suffix('.mov').is_file() and not image.exists()
    a_error = type('Result', (), {'stderr': (proof / 'cooperative-a.stderr').read_bytes()})()
    results['cooperative_writer_rejects_partial_claim'] = {
        'a_exit': a_code, 'b_exit': b_code, **check_staged('cooperative', image, a_error)}

    seed = work / 'source/capture.heic'
    seed_hashes = hashes(seed)
    image = work / 'noncooperative/capture.heic'
    result = convert('noncooperative', image, mode='competitor-before-failed-still-link', seed=seed)
    assert result.returncode == 1 and hashes(image) == seed_hashes
    validate('noncooperative-final-valid', image)
    results['failure_preserves_noncooperating_writer_pair'] = {
        'exit': result.returncode, 'both_competitor_hashes_preserved': True,
        **check_staged('noncooperative', image, result, partial_matches=False)}

    image = work / 'unsupported/capture.heic'
    image.parent.mkdir()
    unrelated = image.parent / 'unowned.txt'
    unrelated.write_bytes(b'KEEP USER BYTES')
    result = convert('unsupported', image, mode='unsupported-movie-link')
    assert result.returncode == 1 and not image.exists() and not image.with_suffix('.mov').exists()
    assert unrelated.read_bytes() == b'KEEP USER BYTES' and not staging(image)
    error = result.stderr.decode()
    assert 'hard-link support required' in error and 'Operation not supported' in error
    results['unsupported_hard_links'] = {'exit': result.returncode, 'no_final_members': True,
                                        'unrelated_bytes_preserved': True,
                                        'explicit_requirement_and_original_os_error': True,
                                        'injected_errno': 'EOPNOTSUPP; not a mounted FAT/exFAT test'}
finally:
    for process in active:
        if process.poll() is None:
            process.kill()
            process.communicate()
    report['fixture_sha256_after'] = sha(fixture)
    report['binary_sha256_after'] = sha(cli)
    report['source_sha256_after'] = {p: sha(root / p) for p in owned_paths}
    shutil.rmtree(work)
    report['scratch_removed'] = not work.exists()
    (proof / 'manifest.json').write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({'proof': str(proof), 'results': results}, ensure_ascii=False, indent=2))
