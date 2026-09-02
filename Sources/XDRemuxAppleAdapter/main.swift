import Foundation
import XDRemuxAppleFeatures

private let schemaVersion = 1

private struct AdapterRequest: Decodable {
    let schemaVersion: Int
    let operation: String

    enum CodingKeys: String, CodingKey {
        case schemaVersion = "schema_version"
        case operation
    }
}

private struct AdapterResponse: Encodable {
    let schemaVersion: Int
    let capabilities: [String]

    enum CodingKeys: String, CodingKey {
        case schemaVersion = "schema_version"
        case capabilities
    }
}

private func fail(_ message: String, status: Int32 = 2) -> Never {
    FileHandle.standardError.write(Data((message + "\n").utf8))
    exit(status)
}

do {
    let input = FileHandle.standardInput.readDataToEndOfFile()
    guard !input.isEmpty else {
        fail("apple adapter request is empty")
    }
    let request = try JSONDecoder().decode(AdapterRequest.self, from: input)
    guard request.schemaVersion == schemaVersion else {
        fail("unsupported apple adapter schema_version \(request.schemaVersion)")
    }
    guard request.operation == "capabilities" else {
        fail("unsupported apple adapter operation \(request.operation)")
    }

    // This target links XDRemuxAppleFeatures. Advertising these facts does not
    // choose a conversion path; the Rust engine remains the policy owner.
    let response = AdapterResponse(
        schemaVersion: schemaVersion,
        capabilities: ["photographic-styles", "portrait"]
    )
    var encoded = try JSONEncoder().encode(response)
    encoded.append(0x0A)
    FileHandle.standardOutput.write(encoded)
} catch {
    fail("invalid apple adapter request: \(error)")
}
