from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]


class PerformanceDesignArchitectureTests(unittest.TestCase):
    def source(self, relative: str) -> str:
        return (ROOT / relative).read_text(encoding="utf-8")

    def test_styles_solver_and_resource_policy_are_in_rust(self) -> None:
        source = self.source("crates/xdremux-runtime/src/apple_styles.rs")
        for marker in (
            "apple_style_fit_global_polynomial",
            "apple_style_monotonic_global_tone_curve",
            "apple_style_property_list",
            "assemble_photographic_styles_heif",
        ):
            self.assertIn(marker, source)
        self.assertNotIn("Command::new", source)
        self.assertNotIn("swift", source.lower())
        self.assertNotIn("python", source.lower())

    def test_rust_style_solver_keeps_sampled_jacobian_bounded(self) -> None:
        source = self.source("crates/xdremux-engine/src/apple_photographic_styles.rs")
        self.assertIn("APPLE_STYLE_REFINEMENT_MAX_PIXELS", source)
        self.assertIn("SampledJacobian", source)
        self.assertIn("Huber", source)

    def test_apple_core_image_context_is_process_scoped(self) -> None:
        source = self.source("Sources/XDRemuxAppleAdapter/CoreImageL8.swift")
        self.assertIn("private let coreImageContext = CIContext", source)
        self.assertEqual(source.count("CIContext(options:"), 1)
        self.assertIn("coreImageContext.render(", source)

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
