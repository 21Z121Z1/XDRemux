#!/usr/bin/env python3
"""Real hdr-look entry points: flat output, native readback, batch failure and reuse."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--adapter', type=Path, required=True)
    parser.add_argument('--input', type=Path, action='append', required=True)
    parser.add_argument('--launcher', type=Path)
    parser.add_argument('--app-smoke', type=Path)
    args = parser.parse_args()
    assert args.binary.is_file() and args.adapter.is_file(), 'build the CLI and native adapter first'
    env = dict(os.environ, XDREMUX_CLI=str(args.binary.resolve()),
               XDREMUX_APPLE_ADAPTER=str(args.adapter.resolve()))
    originals = {p: sha(p) for p in args.input}
    with tempfile.TemporaryDirectory(prefix='xdremux-hdr-look-native-') as temporary:
        root = Path(temporary)
        inputs = root / '原图 folder'
        inputs.mkdir()
        for i, photo in enumerate(args.input):
            nested = inputs / str(i)
            nested.mkdir()
            shutil.copyfile(photo, nested / 'same name.HEIC')
        (inputs / 'broken.heic').write_bytes(b'malformed input')
        out = root / '照片 only'
        command = [str(args.binary), 'hdr-look', str(inputs), '--recursive',
                   '--output-dir', str(out), '--max-edge', '512', '--json']
        done = subprocess.run(command, env=env, capture_output=True, text=True, timeout=1200)
        assert done.returncode == 1 and done.stdout.strip(), (done.stderr, done.stdout)
        receipt = json.loads(done.stdout)
        assert 'manifest' not in receipt and receipt['failures'] == 1
        good = [r for r in receipt['results'] if r['status'] == 'success']
        assert len(good) == len(args.input), receipt
        files = sorted(out.iterdir())
        assert len(files) == len(good) and all(p.name.endswith('.hdr-look.heic') for p in files)
        assert all(p.is_file() for p in files), files
        old = {p: sha(p) for p in files}
        again = subprocess.run(command, env=env, capture_output=True, text=True, timeout=300)
        assert again.returncode == 1
        assert json.loads(again.stdout)['failures'] == len(args.input) + 1
        assert {p: sha(p) for p in out.iterdir()} == old

        readback = root / 'readback'
        readback.mkdir()
        request = dict(schema_version=2, operation='styles-readback', input_path=good[0]['output'],
                       output_path=str(readback), width=32, height=32, target_headroom=8)
        native = subprocess.run([str(args.adapter)], input=json.dumps(request), env=env,
                                text=True, capture_output=True, timeout=300)
        assert native.returncode == 0, native.stderr
        facts = json.loads(native.stdout)
        assert abs(facts['hdr_content_headroom'] - 8) < 1e-5, facts

        if args.launcher:
            single = root / 'launcher output'
            launcher_env = env.copy()
            launcher_env.pop('XDREMUX_CLI', None)
            launcher_env.pop('XDREMUX_APPLE_ADAPTER', None)
            done = subprocess.run(['/bin/zsh', str(args.launcher), str(args.input[0]),
                                   '--output-dir', str(single), '--json'], env=launcher_env,
                                  text=True, capture_output=True, timeout=1200)
            assert done.returncode == 0, done.stderr
            launched = json.loads(done.stdout)
            assert launched['max_edge'] == 0 and launched['headroom'] == 8 and launched['failures'] == 0
            assert len(list(single.iterdir())) == 1 and next(single.iterdir()).is_file()
            assert next(single.iterdir()).name.endswith('.hdr-look.heic')
        if args.app_smoke:
            app = root / 'app output'
            done = subprocess.run([str(args.app_smoke), str(args.input[0]), str(app), '--hdr-look'],
                                  env=env, capture_output=True, text=True, timeout=1200)
            assert done.returncode == 0, done.stderr
            assert len(list(app.iterdir())) == 1 and next(app.iterdir()).is_file()
        assert all(sha(p) == digest for p, digest in originals.items()), 'original was modified'
        print(json.dumps(dict(batch_successes=len(good), batch_failures=1, no_sidecars=True,
                              preserved_originals=True, existing_outputs_preserved=True,
                              native_headroom=facts['hdr_content_headroom'],
                              full_resolution_launcher=bool(args.launcher),
                              app_bridge=bool(args.app_smoke))))


if __name__ == '__main__':
    main()
