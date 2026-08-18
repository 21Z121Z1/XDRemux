#!/usr/bin/env python3
from pathlib import Path

root = Path(__file__).resolve().parents[1]
encoder_path = root / "Sources" / "XDRemuxCore" / "HEIF" / "DirectTiledHEVCGainMapEncoder.swift"
text = encoder_path.read_text(encoding="utf-8")

old = '''        try imageData.write(to: inputURL, options: .atomic)
'''
new = '''        // The input is UUID-scoped scratch consumed immediately by the helper; durability and
        // atomic replacement are not part of this internal transport contract.
        try imageData.write(to: inputURL)
'''
if old in text:
    text = text.replace(old, new, 1)
elif new not in text:
    raise RuntimeError("compressed helper scratch write not found")

old = '''        let tilePayloads = try idrTilePayloads(from: Data(contentsOf: annexBURL))
'''
new = '''        let tilePayloads = try idrTilePayloads(
            from: Data(contentsOf: annexBURL, options: [.mappedIfSafe])
        )
'''
if old in text:
    text = text.replace(old, new, 1)
elif new not in text:
    raise RuntimeError("Annex-B mapping call not found")

old = '''    private static func idrTilePayloads(from annexB: Data) throws -> [Data] {
        let bytes = [UInt8](annexB)
        var starts: [(offset: Int, length: Int)] = []
        var index = 0
        while index + 3 < bytes.count {
            if bytes[index] == 0, bytes[index + 1] == 0,
               bytes[index + 2] == 0, bytes[index + 3] == 1 {
                starts.append((index, 4))
                index += 4
            } else if bytes[index] == 0, bytes[index + 1] == 0, bytes[index + 2] == 1 {
                starts.append((index, 3))
                index += 3
            } else {
                index += 1
            }
        }
        var payloads: [Data] = []
        for position in starts.indices {
            let start = starts[position].offset + starts[position].length
            let end = position + 1 < starts.count ? starts[position + 1].offset : bytes.count
            guard start < end else { continue }
            let type = (bytes[start] >> 1) & 0x3f
            guard type == 19 || type == 20 else { continue }
            var payload = Data()
            appendUInt32BE(end - start, to: &payload)
            payload.append(contentsOf: bytes[start..<end])
            payloads.append(payload)
        }
        guard !payloads.isEmpty else {
            throw CLIError.invalidContainer("private tile encoder emitted no HEVC IDR samples")
        }
        return payloads
    }
'''
new = '''    private static func idrTilePayloads(from annexB: Data) throws -> [Data] {
        var starts: [(offset: Int, length: Int)] = []
        starts.reserveCapacity(64)
        var index = annexB.startIndex
        while index + 3 < annexB.endIndex {
            if annexB[index] == 0, annexB[index + 1] == 0,
               annexB[index + 2] == 0, annexB[index + 3] == 1 {
                starts.append((index, 4))
                index += 4
            } else if annexB[index] == 0, annexB[index + 1] == 0, annexB[index + 2] == 1 {
                starts.append((index, 3))
                index += 3
            } else {
                index += 1
            }
        }
        var payloads: [Data] = []
        payloads.reserveCapacity(starts.count)
        for position in starts.indices {
            let start = starts[position].offset + starts[position].length
            let end = position + 1 < starts.count ? starts[position + 1].offset : annexB.endIndex
            guard start < end else { continue }
            let type = (annexB[start] >> 1) & 0x3f
            guard type == 19 || type == 20 else { continue }
            var payload = Data()
            payload.reserveCapacity(4 + end - start)
            appendUInt32BE(end - start, to: &payload)
            payload.append(contentsOf: annexB[start..<end])
            payloads.append(payload)
        }
        guard !payloads.isEmpty else {
            throw CLIError.invalidContainer("private tile encoder emitted no HEVC IDR samples")
        }
        return payloads
    }
'''
if old in text:
    text = text.replace(old, new, 1)
elif new not in text:
    raise RuntimeError("Annex-B parser body not found")

encoder_path.write_text(text, encoding="utf-8")

arch_path = root / "Tests" / "test_performance_design.py"
arch = arch_path.read_text(encoding="utf-8")
needle = '''        self.assertIn('pathExtension: "raw"', wrapper)
'''
addition = needle + '''        self.assertNotIn("let bytes = [UInt8](annexB)", wrapper)
        self.assertIn("Data(contentsOf: annexBURL, options: [.mappedIfSafe])", wrapper)
'''
if 'self.assertNotIn("let bytes = [UInt8](annexB)"' not in arch:
    if needle not in arch:
        raise RuntimeError("direct encoder architecture assertion anchor missing")
    arch_path.write_text(arch.replace(needle, addition, 1), encoding="utf-8")

print("removed compressed Gain Map scratch/Annex-B ownership overhead")
