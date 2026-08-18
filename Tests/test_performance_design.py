from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class PerformanceDesignArchitectureTests(unittest.TestCase):
    def source(self, relative: str) -> str:
        return (ROOT / relative).read_text(encoding="utf-8")

    def test_gain_map_hot_loop_borrows_mask_storage(self) -> None:
        source = self.source("Sources/XDRemuxCore/HDR/HDRPipeline.swift")
        self.assertNotIn("let maskBytes = [UInt8](mask.data)", source)
        self.assertIn("mask.data.withUnsafeBytes { inputRawBuffer in", source)

    def test_raw_tiff_reader_keeps_data_storage(self) -> None:
        source = self.source("Sources/XDRemuxCore/RAW/CoreImageRAW.swift")
        self.assertNotIn("let bytes = [UInt8](data)", source)
        self.assertIn("let bytes: Data", source)

    def test_style_jacobian_is_sampled_and_flat(self) -> None:
        source = self.source(
            "Sources/XDRemuxAppleFeatures/PhotographicStyles/ConstrainedPolynomialStyleDataProducer.swift"
        )
        self.assertNotIn("var derivatives: [[Float]]", source)
        self.assertNotIn("zip(rendered.rgb, currentRaster.rgb).map", source)
        self.assertIn("private struct SampledJacobian", source)
        self.assertIn("var values: [Float]", source)
        self.assertNotIn("let derivativeRasters = try Self.render", source)
        self.assertNotIn("let initializationRasters = try Self.render", source)
        self.assertNotIn("let lineSearchRasters = try Self.render", source)
        self.assertNotIn("Self.metrics(rendered, target).dictionary", source)
        self.assertIn("try Self.executeRenderRequests", source)
        self.assertNotIn("var renderCache: [String: Raster]", source)

    def test_style_candidate_does_not_copy_whole_heic_in_swift(self) -> None:
        source = self.source(
            "Sources/XDRemuxAppleFeatures/PhotographicStyles/ConstrainedPolynomialStyleDataProducer.swift"
        )
        self.assertNotIn("var output = source", source)
        self.assertIn("fileManager.copyItem(at: heicURL, to: outputURL)", source)
        self.assertIn("try handle.seek(toOffset: UInt64(styleOffset))", source)
        self.assertIn("permissions & 0o200 == 0", source)
        self.assertIn(".posixPermissions: permissions | 0o200", source)

    def test_direct_raster_encoder_uses_raw_transport(self) -> None:
        wrapper = self.source("Sources/XDRemuxCore/HEIF/DirectTiledHEVCGainMapEncoder.swift")
        helper = self.source("Sources/XDRemuxCore/Resources/Native/apple_vt_hevc_encoder.swift")
        self.assertNotIn("CGImageDestinationCreateWithData", wrapper)
        self.assertIn('pathExtension: "raw"', wrapper)
        self.assertNotIn("let bytes = [UInt8](annexB)", wrapper)
        self.assertIn("Data(contentsOf: annexBURL, options: [.mappedIfSafe])", wrapper)
        self.assertIn("RawRasterDescriptor", helper)
        self.assertIn("Data(contentsOf: url, options: [.mappedIfSafe])", helper)

    def test_app_queue_status_projection_avoids_filter_arrays(self) -> None:
        source = self.source("apps/macos/XDRemuxApp/Sources/XDRemuxViewModel.swift")
        self.assertNotIn("queue.filter { $0.status.isTerminal }.count", source)
        for status in (
            ".pending", ".running", ".converted", ".skippedExisting", ".failed", ".cancelled"
        ):
            self.assertNotIn(f"queue.filter {{ $0.status == {status} }}.count", source)
        self.assertNotIn("queue.firstIndex(where: { $0.id == item.id })", source)
        self.assertIn("struct ConversionQueueStatusCounts", source)
        self.assertIn("queueIndexByID.reserveCapacity(queue.count)", source)


if __name__ == "__main__":
    unittest.main()
