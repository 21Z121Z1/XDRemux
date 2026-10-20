#!/usr/bin/env python3
"""Recheck persisted native evidence; no network or Apple implementation executes."""
import argparse
import hashlib
import json
from pathlib import Path
import tempfile

from read_texture_native_log import extract


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", type=Path, default=Path("research/texture-style/evidence"))
    args = parser.parse_args()
    index = json.loads((args.evidence/"native-readback-index.json").read_text())
    for row in index["jobs"]:
        source = args.evidence/f'run{row["run_id"]}'/f'job{row["job_id"]}.log'
        if hashlib.sha256(source.read_bytes()).hexdigest() != row["log_sha256"]:
            raise ValueError(f"whole native job log changed: {source}")
    jobs = payloads = 0
    for receipt in sorted(args.evidence.rglob("log-readback.json")):
        source = receipt.parent.with_suffix(".log")
        with tempfile.TemporaryDirectory(prefix="xd-texture-log-") as temporary:
            recovered = Path(temporary)/"recovered"
            summary = extract(source.read_bytes().decode("utf-8"), recovered)
            if summary != json.loads(receipt.read_text()):
                raise ValueError(f"persisted summary differs from source log: {receipt}")
            for file in recovered.rglob("*.json"):
                saved = receipt.parent/file.relative_to(recovered)
                if saved.read_bytes() != file.read_bytes():
                    raise ValueError(f"persisted native JSON differs from manifest-checked log: {saved}")
            jobs += 1
            payloads += summary["verified_payload_count"]
    if jobs != index["manifests_verified"] or payloads != index["native_JSON_payloads_verified"]:
        raise ValueError("readback job/payload totals disagree with source index")
    print(json.dumps({"complete_job_logs": len(index["jobs"]), "verified_manifests": jobs,
                      "verified_logged_native_JSON_payloads": payloads,
                      "ZIP_bytes_verified": False, "native_calls_reexecuted": False,
                      "scope": "persisted actual native log integrity only; not image eligibility, rendering or Photos"}))


if __name__ == "__main__":
    main()
