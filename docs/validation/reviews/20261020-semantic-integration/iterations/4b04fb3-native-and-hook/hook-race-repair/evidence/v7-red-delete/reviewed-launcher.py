#!/opt/codex/runtimes/codex-primary-runtime/dependencies/python/bin/python3
"""Temporary pinned pre-push chain. It does not push or modify Git config."""
import hashlib
import os
from pathlib import Path
import signal
import subprocess
import sys

MANIFEST = '/workspace/xdremux-audit/cleanup-guard/hook-race-repair/local-fixtures-74goh9xg/v7-red-delete/fixture-approved.json'
MANIFEST_SHA256 = '9a56a2e4c1cbbfe8aaf3864d0d1e6f1ebee5b609960ed8dd5eb3fb89fae7af45'
GUARD = '/workspace/xdremux-audit/cleanup-guard/guard.py'
GUARD_SHA256 = '2f851aaa0f7824e117cfcef55d4efe52cfbefda94c27f7f61e97a16c8d17fd33'
ORIGINAL_HOOKS_DIRECTORY = '/workspace/xdremux-audit/cleanup-guard/hook-race-repair/local-fixtures-74goh9xg/v7-red-delete/client.git/hooks'
ORIGINAL_HOOKS = {'applypatch-msg.sample': {'executable': True,
                           'sha256': '0223497a0b8b033aa58a3a521b8629869386cf7ab0e2f101963d328aa62193f7'},
 'commit-msg.sample': {'executable': True,
                       'sha256': '1f74d5e9292979b573ebd59741d46cb93ff391acdd083d340b94370753d92437'},
 'fsmonitor-watchman.sample': {'executable': True,
                               'sha256': 'e0549964e93897b519bd8e333c037e51fff0f88ba13e086a331592bf801fa1d0'},
 'post-update.sample': {'executable': True,
                        'sha256': '81765af2daef323061dcbc5e61fc16481cb74b3bac9ad8a174b186523586f6c5'},
 'pre-applypatch.sample': {'executable': True,
                           'sha256': 'e15c5b469ea3e0a695bea6f2c82bcf8e62821074939ddd85b77e0007ff165475'},
 'pre-commit.sample': {'executable': True,
                       'sha256': '57185b7b9f05239d7ab52db045f5b89eb31348d7b2177eab214f5eb872e1971b'},
 'pre-merge-commit.sample': {'executable': True,
                             'sha256': 'd3825a70337940ebbd0a5c072984e13245920cdf8898bd225c8d27a6dfc9cb53'},
 'pre-push': {'executable': True,
              'sha256': '0ee46ed95f5f4302a3baaad53d0ea73f916fff0bef9f4f49068dd034b6ea30f6'},
 'pre-push.sample': {'executable': True,
                     'sha256': 'ecce9c7e04d3f5dd9d8ada81753dd1d549a9634b26770042b58dda00217d086a'},
 'pre-rebase.sample': {'executable': True,
                       'sha256': '4febce867790052338076f4e66cc47efb14879d18097d1d61c8261859eaaa7b3'},
 'pre-receive.sample': {'executable': True,
                        'sha256': 'a4c3d2b9c7bb3fd8d1441c31bd4ee71a595d66b44fcf49ddb310252320169989'},
 'prepare-commit-msg.sample': {'executable': True,
                               'sha256': 'e9ddcaa4189fddd25ed97fc8c789eca7b6ca16390b2392ae3276f0c8e1aa4619'},
 'push-to-checkout.sample': {'executable': True,
                             'sha256': 'a53d0741798b287c6dd7afa64aee473f305e65d3f49463bb9d7408ec3b12bf5f'},
 'sendemail-validate.sample': {'executable': True,
                               'sha256': '44ebfc923dc5466bc009602f0ecf067b9c65459abfe8868ddc49b78e6ced7a92'},
 'update.sample': {'executable': True,
                   'sha256': '8d5f2fa83e103cf08b57eaa67521df9194f45cbdbcb37da52ad586097a14d106'}}


def hook_snapshot(directory):
    root = Path(directory)
    if not root.is_dir():
        raise ValueError("original hooks directory disappeared")
    return {p.name: {"sha256": hashlib.sha256(p.read_bytes()).hexdigest(),
                     "executable": os.access(p, os.X_OK)}
            for p in root.iterdir() if p.is_file()}


try:
    payload = sys.stdin.buffer.read()
    if hook_snapshot(ORIGINAL_HOOKS_DIRECTORY) != ORIGINAL_HOOKS:
        raise ValueError("original hooks changed after preparation; re-review the chain")
    original = Path(ORIGINAL_HOOKS_DIRECTORY) / "pre-push"
    if original.exists():
        if not original.is_file() or not os.access(original, os.X_OK):
            raise ValueError("existing pre-push is not an executable file; do not bypass it")
        result = subprocess.run([str(original), *sys.argv[1:]], input=payload)
        if result.returncode < 0:
            if -result.returncode not in (signal.SIGKILL, signal.SIGSTOP):
                signal.signal(-result.returncode, signal.SIG_DFL)
            os.kill(os.getpid(), -result.returncode)
        if result.returncode:
            sys.exit(result.returncode)
    guard_bytes = Path(GUARD).read_bytes()
    manifest_bytes = Path(MANIFEST).read_bytes()
    if hashlib.sha256(guard_bytes).hexdigest() != GUARD_SHA256:
        raise ValueError("guard SHA-256 differs from reviewed launcher")
    if hashlib.sha256(manifest_bytes).hexdigest() != MANIFEST_SHA256:
        raise ValueError("manifest SHA-256 differs from reviewed launcher")
    namespace = {"__name__": "pinned_cleanup_guard", "__file__": GUARD}
    # Execute precisely the bytes just hashed, rather than reopening a changed path.
    exec(compile(guard_bytes, GUARD, "exec"), namespace)
    refs = namespace["validate"](manifest_bytes, sys.argv[1:], payload)
    print("cleanup guard approved advertised deletion batch: " + ", ".join(refs), file=sys.stderr)
except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
    print(f"cleanup guard rejected: {error}", file=sys.stderr)
    sys.exit(1)
