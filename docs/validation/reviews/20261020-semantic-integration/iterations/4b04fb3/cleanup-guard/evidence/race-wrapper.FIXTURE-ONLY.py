#!/opt/codex/runtimes/codex-primary-runtime/dependencies/python/bin/python3
from pathlib import Path
import subprocess, sys
payload = sys.stdin.buffer.read()
result = subprocess.run(['/workspace/xdremux-audit/cleanup-guard/local-fixtures-wum6z4xv/change-after-advertisement/guarded-hooks/pre-push', *sys.argv[1:]], input=payload)
if result.returncode:
 sys.exit(result.returncode)
# Fixture-only writer: the server has already advertised BACKED to this push.
subprocess.run(["git", "--git-dir=/workspace/xdremux-audit/cleanup-guard/local-fixtures-wum6z4xv/change-after-advertisement/remote.git", "update-ref", 'refs/heads/fixture/a', '89d806c8aec961a7991eb3e6d6e2b1370aed8791', '1b2c07da2cb2b0c89c64ca546148fe5d68d75a97'], check=True)
Path('/workspace/xdremux-audit/cleanup-guard/local-fixtures-wum6z4xv/change-after-advertisement/writer-after-guard.txt').write_text("guard approved BACKED, writer changed A to MAIN before client update request\n")
