import Foundation
import XCTest
import XDRemuxCore
@testable import XDRemuxCLI

final class LocalizationTests: XCTestCase {
    func testAutomaticLanguageSelection() {
        XCTAssertEqual(resolve(preferred: ["zh-Hans-CN"]), .simplifiedChinese)
        XCTAssertEqual(resolve(preferred: ["zh-CN"]), .simplifiedChinese)
        XCTAssertEqual(resolve(preferred: ["en-US"]), .english)
        XCTAssertEqual(resolve(preferred: ["fr-FR"]), .english)
    }

    func testExplicitLanguageOverridesEnvironment() {
        XCTAssertEqual(
            Localizer.resolveLanguage(
                requested: .english,
                environmentValue: "zh-Hans",
                preferredLanguages: ["zh-CN"]
            ),
            .english
        )
    }

    func testEnvironmentOverridesSystemLanguage() {
        XCTAssertEqual(
            Localizer.resolveLanguage(
                requested: nil,
                environmentValue: "zh-Hans",
                preferredLanguages: ["en-GB"]
            ),
            .simplifiedChinese
        )
    }

    func testLocalizedFormatsHaveMatchingPlaceholderCounts() {
        let english = Localizer(
            requested: .english,
            environment: [:],
            preferredLanguages: []
        )
        let chinese = Localizer(
            requested: .simplifiedChinese,
            environment: [:],
            preferredLanguages: []
        )
        let parameterizedKeys: [MessageKey] = [
            .statusSingleCompleted,
            .statusSingleSkipped,
            .statusBatchStarted,
            .statusBatchProgress,
            .statusBatchCompleted,
            .statusFileCompleted,
            .statusFileSkipped,
            .statusFileFailed,
            .statusWarningPlain,
            .statusWarning,
            .statusError,
            .statusRecovery,
            .statusFailureReport,
            .argumentMissing,
            .argumentUnknown,
            .argumentInvalid,
            .argumentInvalidCommand,
            .argumentIncompatible,
        ]

        for key in parameterizedKeys {
            XCTAssertEqual(
                placeholderCount(in: english.formatString(for: key)),
                placeholderCount(in: chinese.formatString(for: key)),
                "placeholder mismatch for \(key.rawValue)"
            )
        }
    }

    func testEnglishCanBeSelectedOnChineseSystem() {
        let localizer = Localizer(
            requested: .english,
            environment: ["XDREMUX_LANGUAGE": "zh-Hans"],
            preferredLanguages: ["zh-CN"]
        )

        XCTAssertEqual(localizer.text(.phaseReadingSource), "Reading source")
    }

    private func resolve(preferred: [String]) -> OutputLanguage {
        Localizer.resolveLanguage(
            requested: nil,
            environmentValue: nil,
            preferredLanguages: preferred
        )
    }

    private func placeholderCount(in format: String) -> Int {
        var count = 0
        var index = format.startIndex
        while index < format.endIndex {
            guard format[index] == "%" else {
                index = format.index(after: index)
                continue
            }
            let next = format.index(after: index)
            if next < format.endIndex, format[next] == "%" {
                index = format.index(after: next)
            } else {
                count += 1
                index = next
            }
        }
        return count
    }
}
