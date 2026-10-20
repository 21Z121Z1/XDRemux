#!/usr/bin/env python3
"""Run the current behavioral regressions against the immutable 4b owner."""
import importlib.util
from pathlib import Path
import sys
import unittest

repo = Path("/workspace/XDRemux-integration")
snapshot = Path("/workspace/xdremux-audit/tone-checker-baseline-4b.py")
sys.path.insert(0, str(repo))
spec = importlib.util.spec_from_file_location("tone_baseline_4b", snapshot)
baseline = importlib.util.module_from_spec(spec)
spec.loader.exec_module(baseline)
from tests import test_style_tone_response

test_style_tone_response.tone = baseline
names = (
    "test_reference_pass_cannot_admit_an_empty_product_sweep",
    "test_all_requested_tones_and_photos_are_required",
    "test_failed_process_or_model_comparison_cannot_reuse_present_renders",
    "test_host_gate_requires_a_successful_nonempty_native_comparison",
)
suite = unittest.TestSuite(test_style_tone_response.StyleToneResponseTests(name) for name in names)
result = unittest.TextTestRunner(verbosity=2).run(suite)
sys.exit(0 if result.wasSuccessful() else 1)
