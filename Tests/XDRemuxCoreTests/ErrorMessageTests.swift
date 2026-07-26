import Foundation
import XCTest
@testable import XDRemuxCore

/// Error text is a product surface. These contracts keep the two failures a
/// user actually hits — wrong file, or a file already converted — phrased in
/// terms of what happened rather than which private OPPO block was missing,
/// and keep list output to one line per file.
final class ErrorMessageTests: XCTestCase {
    private let sample = URL(fileURLWithPath: "/tmp/IMG_0001.heic")

    func testNotAProXDRPhotoLeadsWithTheUserFacingReason() {
        let text = XDRemuxError.notAProXDRPhoto(
            sample,
            detail: "no OPPO Local HDR payload found (no QTI extension marker)"
        ).description

        XCTAssertTrue(text.hasPrefix("not a ProXDR photo: /tmp/IMG_0001.heic"))
        XCTAssertTrue(text.contains("nothing to convert"), "must say why there is no work to do")
        XCTAssertTrue(text.contains("no QTI extension marker"), "must keep the detail for bug reports")
        XCTAssertFalse(
            text.contains("local.hdr.meta.data"),
            "the private block name must not be the headline a user reads"
        )
    }

    func testAlreadyConvertedSaysThereIsNothingLeftToDo() {
        let text = XDRemuxError.alreadyConverted(sample).description

        XCTAssertTrue(text.hasPrefix("already converted: /tmp/IMG_0001.heic"))
        XCTAssertTrue(text.contains("ISO 21496-1"))
        XCTAssertTrue(text.contains("would not change anything"))
    }

    func testHeadlineIsOneLineAndDropsTheRedundantPath() {
        for error in [
            XDRemuxError.notAProXDRPhoto(sample, detail: "whatever"),
            XDRemuxError.alreadyConverted(sample),
        ] {
            let headline = error.headline
            XCTAssertFalse(headline.contains("\n"), "list output must stay one line")
            XCTAssertFalse(
                headline.contains(sample.path),
                "the caller already prints the file name"
            )
            XCTAssertFalse(headline.isEmpty)
        }
    }

    func testEveryHeadlineIsASingleNonEmptyLine() {
        let errors: [XDRemuxError] = [
            .inputNotFound(sample),
            .unableToRead(sample),
            .qtiMarkerNotFound,
            .manifestNotFound,
            .invalidLHDR("mask is empty"),
            .unableToDecodeMask(sample),
            .unableToLoadBaseImage(sample),
            .outputVerificationFailed(sample),
            .portraitPrerequisitesMissing("--apple-portrait requires rear.depth"),
            .batchFailed(failures: 3, checkpoint: sample),
            .categorizationFailed(failures: 2),
        ]
        for error in errors {
            XCTAssertFalse(error.headline.contains("\n"), "\(error) headline must be one line")
            XCTAssertFalse(error.headline.isEmpty, "\(error) headline must not be empty")
        }
    }

    func testBatchFailureTellsTheUserHowToRetry() {
        let text = XDRemuxError.batchFailed(
            failures: 3,
            checkpoint: URL(fileURLWithPath: "/tmp/.xdremux-batch.jsonl")
        ).description

        XCTAssertTrue(text.contains("3 file(s) failed"))
        XCTAssertTrue(text.contains("run the same command again"), "must state the recovery step")
    }

    func testPortraitPrerequisitesDoNotClaimTheContainerIsInvalid() {
        let text = XDRemuxError.portraitPrerequisitesMissing(
            "--apple-portrait requires rear.depth + rear.depth.config + src.image"
        ).description

        XCTAssertTrue(text.hasPrefix("not an OPPO portrait photo:"))
        XCTAssertFalse(
            text.contains("invalid HEIC container"),
            "a non-portrait photo is not a damaged file"
        )
    }
}
