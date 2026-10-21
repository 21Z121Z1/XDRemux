from pathlib import Path
import hashlib
import json
import os
import subprocess
import sys
import tempfile

from PIL import Image, ImageDraw

REPO = Path('/workspace/XDRemux-integration')
sys.path.insert(0, str(REPO))
from scripts.apple.pvt_native_import.check_native_import import wrap_motion_photo

root = Path(tempfile.mkdtemp(prefix='pvt-synthetic-cli-', dir='/workspace/xdremux-audit'))
cli = REPO / 'target/debug/xdremux'
report = {'scope': 'Linux synthetic Motion Photo to current CLI PVT; not AVFoundation, native import or device evidence',
          'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=REPO, text=True).strip(),
          'source_status': subprocess.check_output(['git', 'status', '--porcelain'], cwd=REPO, text=True).strip(),
          'binary_sha256': hashlib.sha256(cli.read_bytes()).hexdigest(), 'commands': [], 'passed': False}
def run(name, argv):
    result = subprocess.run(list(map(str, argv)), cwd=REPO, capture_output=True, timeout=120)
    (root / (name + '.stdout')).write_bytes(result.stdout)
    (root / (name + '.stderr')).write_bytes(result.stderr)
    report['commands'].append({'name': name, 'argv': list(map(str, argv)), 'exit': result.returncode})
    if result.returncode:
        raise RuntimeError(name + ' failed; exact logs retained')
    return result.stdout
try:
    assert report['head'] == '04c09aac1b0cabdbd412ab10d071f74120b7bff6' and not report['source_status']
    image = Image.new('RGB', (256, 256), (20, 46, 82))
    ImageDraw.Draw(image).rectangle((80, 64, 112, 192), fill=(230, 166, 31))
    jpeg = root / 'still.jpg'
    image.save(jpeg, quality=90)
    movie = root / 'motion.mp4'
    run('generate-movie', ['ffmpeg', '-v', 'error', '-f', 'lavfi', '-i', 'color=c=0x142e52:s=256x256:r=30:d=2',
                           '-c:v', 'libx264', '-profile:v', 'main', '-pix_fmt', 'yuv420p', '-g', '30', '-an', movie])
    source = root / 'source.jpg'
    source.write_bytes(wrap_motion_photo(jpeg.read_bytes(), movie.read_bytes()))
    before = hashlib.sha256(source.read_bytes()).hexdigest()
    run('inspect', [cli, 'inspect', source, '--json'])
    package = root / 'capture.pvt'
    run('convert', [cli, 'convert', '--input', source, '--output', package, '--live-photo-output', 'pvt'])
    validation = json.loads(run('validate', [cli, 'validate', package, '--json']))
    assert validation['valid'] and validation['kind'] == 'live-photo'
    assert hashlib.sha256(source.read_bytes()).hexdigest() == before
    report.update(passed=True, source_sha256=before, source_unchanged=True, validation=validation,
                  package_hashes={p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in package.iterdir()})
except Exception as error:
    report['error'] = repr(error)
finally:
    (root / 'manifest.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'passed': report['passed'], 'manifest': str(root / 'manifest.json'), 'error': report.get('error')}))
sys.exit(0 if report['passed'] else 1)
