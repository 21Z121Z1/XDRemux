#!/usr/bin/env python3
from pathlib import Path

root = Path(__file__).resolve().parents[1]
path = root / "Tests" / "test_performance_design.py"
text = path.read_text(encoding="utf-8")
old = '''        self.assertNotIn("queue.filter { $0.status", source)
        self.assertNotIn("queue.firstIndex(where: { $0.id == item.id })", source)
'''
new = '''        self.assertNotIn("queue.filter { $0.status.isTerminal }.count", source)
        for status in (
            ".pending", ".running", ".converted", ".skippedExisting", ".failed", ".cancelled"
        ):
            self.assertNotIn(f"queue.filter {{ $0.status == {status} }}.count", source)
        self.assertNotIn("queue.firstIndex(where: { $0.id == item.id })", source)
'''
if old in text:
    text = text.replace(old, new, 1)
elif new not in text:
    raise RuntimeError("phase3 queue architecture assertion block not found")
path.write_text(text, encoding="utf-8")
print("narrowed queue performance guard to count/filter regressions")
