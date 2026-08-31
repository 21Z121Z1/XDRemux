import Foundation

/// Migration boundary for product-policy decisions that still live in Swift.
///
/// These helpers are deliberately pure so the current Swift product path and
/// the cross-language plan oracle exercise the same rules. New policy belongs
/// in `xdremux-engine`; this surface exists only to make legacy semantics
/// explicit while the Rust planner takes ownership of the product flow.
package enum LegacyConversionExecutionRoute: String, Sendable {
    case core
    case appleFeatures = "apple-features"
}

package func resolveLegacyConversionExecutionRoute(
    configuration: ConversionConfiguration
) -> LegacyConversionExecutionRoute {
    configuration.appleFeaturesEnabled ? .appleFeatures : .core
}

package func resolveEffectiveInputProcessingBranch(
    requested: InputProcessingBranch,
    oppoCameraTail: OppoCameraTail,
    tmapFormat: TmapFormat
) -> InputProcessingBranch {
    switch oppoCameraTail {
    case .preserve,
         .preserveWithoutPortrait,
         .preserveWithoutPortraitOrPrivateHDR,
         .preserveWithoutPrivateUHDR,
         .preserveWithoutPrivateHDR:
        return .hybrid
    case .off,
         .watermark,
         .compact,
         .preserveNoUHDR,
         .preserveNoHDR:
        return tmapFormat == .strict ? .hybrid : requested
    }
}
