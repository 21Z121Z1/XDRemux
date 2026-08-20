from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]

class ReverseKey1DataAuditTests(unittest.TestCase):
    def test_audit_and_adapter_keep_heldout_closed(self):
        audit = (ROOT / "scripts/audit_reverse_key1_17pro_data.py").read_text()
        adapter = (ROOT / "scripts/evaluate_17pro_scalar_adapter.py").read_text()
        self.assertIn("ineligible train record", audit)
        self.assertIn("'heldout':'not opened'", adapter)
        self.assertIn("parameterCount':1", adapter)

if __name__ == "__main__":
    unittest.main()
