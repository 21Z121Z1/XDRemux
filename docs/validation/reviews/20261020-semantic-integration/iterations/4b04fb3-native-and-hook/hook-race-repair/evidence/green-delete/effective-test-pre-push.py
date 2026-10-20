#!/opt/codex/runtimes/codex-primary-runtime/dependencies/python/bin/python3
import os, runpy, sys
from pathlib import Path
target = '/workspace/xdremux-audit/cleanup-guard/hook-race-repair/guard.py'
def trace(frame, event, arg):
 if event == "line" and frame.f_code.co_filename == target and frame.f_lineno == 85:
  sys.settrace(None)
  previous = Path('/workspace/xdremux-audit/cleanup-guard/hook-race-repair/local-fixtures-74goh9xg/green-delete/client.git/hooks/pre-push')
  if 'delete' == "delete":
   previous.unlink()
  else:
   replacement = previous.with_name("replacement.tmp")
   replacement.write_text("#!/opt/codex/runtimes/codex-primary-runtime/dependencies/python/bin/python3\nfrom pathlib import Path\nimport sys\nPath('/workspace/xdremux-audit/cleanup-guard/hook-race-repair/local-fixtures-74goh9xg/green-delete/replacement-executed.txt').write_text('EXECUTED\\n')\nsys.stdin.buffer.read()\nsys.exit(0)\n")
   replacement.chmod(0o755)
   os.replace(replacement, previous)
  Path('/workspace/xdremux-audit/cleanup-guard/hook-race-repair/local-fixtures-74goh9xg/green-delete/race-interleaving.txt').write_text(
    "Observed original hook snapshot; fixture writer delete before subsequent decision\n")
 return trace
sys.settrace(trace)
runpy.run_path('/workspace/xdremux-audit/cleanup-guard/hook-race-repair/local-fixtures-74goh9xg/green-delete/reviewed-launcher.py', run_name="__main__")
