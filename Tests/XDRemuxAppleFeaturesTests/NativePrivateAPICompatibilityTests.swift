import Foundation
import XCTest
@testable import XDRemuxAppleFeatures

final class NativePrivateAPICompatibilityTests: XCTestCase {
    func testSyntheticPrivateAPICompatibilityMatrix() throws {
        let executable = try AppleNativeToolchain.learnExecutable()
        let result = try AppleNativeToolchain.run(
            executable,
            arguments: ["--self-test-runtime-compat"],
            timeout: 30
        )
        let stderr = String(data: result.stderr, encoding: .utf8) ?? ""
        XCTAssertFalse(result.timedOut, stderr)
        XCTAssertEqual(result.status, 0, stderr)
        let payload = try XCTUnwrap(
            JSONSerialization.jsonObject(with: result.stdout) as? [String: Any]
        )
        XCTAssertEqual(payload["passed"] as? Bool, true)
        let source = try XCTUnwrap(payload["photoEditSource"] as? [String: Any])
        XCTAssertEqual(source["legacy"] as? Bool, true)
        XCTAssertEqual(source["modern"] as? Bool, true)
        XCTAssertEqual(source["missing"] as? Bool, true)
        XCTAssertEqual(source["abiMismatch"] as? Bool, true)
        let apply = try XCTUnwrap(payload["styleApply"] as? [String: Any])
        XCTAssertEqual(apply["legacy"] as? Bool, true)
        XCTAssertEqual(apply["displacement"] as? Bool, true)
        XCTAssertEqual(apply["displacementWasNil"] as? Bool, true)
        XCTAssertEqual(apply["missing"] as? Bool, true)
        XCTAssertEqual(apply["abiMismatch"] as? Bool, true)
    }

    func testHostedRuntimePrivateAPICapability() throws {
        guard let fixture = ProcessInfo.processInfo.environment[
            "XDREMUX_PRIVATE_API_CAPABILITY_FIXTURE"
        ], !fixture.isEmpty else {
            throw XCTSkip("private API capability fixture not configured")
        }
        let executable = try AppleNativeToolchain.learnExecutable()
        let result = try AppleNativeToolchain.run(
            executable,
            arguments: ["--private-api-capabilities", fixture],
            timeout: 30
        )
        let stderr = String(data: result.stderr, encoding: .utf8) ?? ""
        let stdout = String(data: result.stdout, encoding: .utf8) ?? ""
        XCTAssertFalse(result.timedOut, stderr)
        XCTAssertEqual(result.status, 0, "\(stderr)\n\(stdout)")
        let payload = try XCTUnwrap(
            JSONSerialization.jsonObject(with: result.stdout) as? [String: Any]
        )
        XCTAssertEqual(payload["passed"] as? Bool, true)
        let source = try XCTUnwrap(payload["photoEditSource"] as? [String: Any])
        let initializer = try XCTUnwrap(source["initializer"] as? String)
        XCTAssertTrue([
            "initWithURL:type:image:useEmbeddedPreview:",
            "initWithURL:type:useEmbeddedPreview:",
        ].contains(initializer))
        let apply = try XCTUnwrap(payload["styleApply"] as? [String: Any])
        let selector = try XCTUnwrap(apply["selector"] as? String)
        XCTAssertTrue([
            "applyStyle:toImage:thumbnail:target:deltaMap:colorSpace:configuration:tuningParameters:noiseModel:error:",
            "applyStyle:toImage:thumbnail:target:deltaMap:displacement:colorSpace:configuration:tuningParameters:noiseModel:error:",
        ].contains(selector))
    }
}
