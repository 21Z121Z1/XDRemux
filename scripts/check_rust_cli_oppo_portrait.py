#!/usr/bin/env python3
"""Real CLI/native-resource regression with a proxy that rejects all Vision calls."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def run(args, **kwargs):
    return subprocess.run(args, cwd=ROOT, check=True, text=True, **kwargs)


def main():
    if sys.platform != "darwin":
        raise SystemExit("OPPO-native Portrait currently requires macOS ImageIO/Core Image")
    run(["swift", "build", "--product", "xdremux-apple-adapter"])
    bin_path = run(["swift", "build", "--show-bin-path"], capture_output=True).stdout.strip()
    adapter = str(Path(bin_path) / "xdremux-apple-adapter")
    run(["cargo", "build", "--locked", "-p", "xdremux-cli"])
    cli = str(ROOT / "target/debug/xdremux")
    fixtures = [ROOT / "fixtures/proxdr/oppo/find-x9-ultra" / f"uhdr-portrait-{i:02}.heic" for i in [1, 2]]
    with tempfile.TemporaryDirectory(prefix="xdremux-native-portrait-") as folder:
        temp = Path(folder)
        proxy = temp / "adapter-proxy"
        trace = temp / "operations.jsonl"
        # Each Rust adapter session receives its own proxy/child pair. The trace
        # uses one append write per operation so concurrent batch jobs can share it.
        proxy.write_text("#!" + sys.executable + "\n" + '''
import json, os, subprocess, sys
child = subprocess.Popen([os.environ["XDREMUX_TEST_REAL_ADAPTER"], *sys.argv[1:]],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
try:
    for line in sys.stdin:
        op = json.loads(line)["operation"]
        fd = os.open(os.environ["XDREMUX_TEST_OPERATION_TRACE"], os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
        try: os.write(fd, (json.dumps(op) + "\\n").encode())
        finally: os.close(fd)
        if "vision" in op.lower():
            raise SystemExit("OPPO-native path attempted Vision: " + op)
        child.stdin.write(line)
        child.stdin.flush()
        response = child.stdout.readline()
        if not response: raise SystemExit("adapter closed without response")
        sys.stdout.write(response)
        sys.stdout.flush()
finally:
    child.stdin.close()
    try: child.wait(timeout=5)
    except subprocess.TimeoutExpired:
        child.kill()
        child.wait()
''')
        proxy.chmod(0o700)
        env = dict(os.environ, XDREMUX_APPLE_ADAPTER=str(proxy),
                   XDREMUX_TEST_REAL_ADAPTER=adapter, XDREMUX_TEST_OPERATION_TRACE=str(trace),
                   XDREMUX_APPLE_ADAPTER_TEST_EXECUTABLE=str(proxy))
        run(["cargo", "test", "--locked", "-p", "xdremux-runtime", "--test", "apple_adapter",
             "oppo_native_portrait_preserves_only_available_source_resources", "--", "--exact"], env=env)

        def facts(path):
            request = {"schema_version": 2, "operation": "imageio-auxiliary-facts", "input_path": str(path)}
            result = run([adapter], input=json.dumps(request) + "\n", capture_output=True)
            data = json.loads(result.stdout)["auxiliary"]
            for key in ["iso_gain_map", "disparity", "focus_metadata"]:
                assert data[key] is True, data
            for key in ["skin_matte", "teeth_matte", "glasses_matte"]:
                assert data[key] is False, data
            return data

        single_facts = []
        for i, fixture in enumerate(fixtures):
            output = temp / f"single-{i}.heic"
            run([cli, "convert", "--input", str(fixture), "--output", str(output), "--apple-portrait-oppo"], env=env)
            run([cli, "validate", str(output)], env=env)
            single_facts.append(facts(output))
        assert not single_facts[0]["portrait_effects_matte"], single_facts[0]
        for jobs in [1, 2]:
            outdir = temp / f"batch-{jobs}"
            outdir.mkdir()
            args = [cli, "batch", "--output-dir", str(outdir), "--apple-portrait-oppo", "--jobs", str(jobs), "--json"]
            for fixture in fixtures:
                args += ["--input", str(fixture)]
            run(args, env=env)
            for fixture, expected in zip(fixtures, single_facts):
                output = outdir / (fixture.stem + ".xdremux.heic")
                run([cli, "validate", str(output)], env=env)
                assert facts(output) == expected
        sentinel = temp / "preserved.heic"
        sentinel.write_bytes(b"existing output must survive failed preflight")
        unsupported = ROOT / "fixtures/proxdr/oppo/find-x9-ultra/uhdr-hr-01.heic"
        assert unsupported.is_file(), unsupported
        failed = subprocess.run([cli, "convert", "--input", str(unsupported), "--output", str(sentinel),
                                 "--apple-portrait-oppo"], cwd=ROOT, env=env, capture_output=True)
        assert failed.returncode != 0
        assert sentinel.read_bytes() == b"existing output must survive failed preflight"
        operations = [json.loads(line) for line in trace.read_text().splitlines()]
        assert "imageio-write-auxiliary" in operations
        assert not any("vision" in op.lower() for op in operations)
        print("PASS OPPO-native Portrait: runtime, single, serial/parallel batch, atomic failure, no Vision calls")
        print(json.dumps(single_facts, sort_keys=True))


if __name__ == "__main__":
    main()
