import Foundation
import XDRemuxCore

private struct PlanCase: Decodable {
    let name: String
    let oppoCompatibility: String?
    let inputProcessingBranch: String?
    let oppoCameraTail: String?
    let tmapFormat: String?
    let applePhotographicStyles: Bool?
    let applePortrait: Bool?

    enum CodingKeys: String, CodingKey {
        case name
        case oppoCompatibility = "oppo_compatibility"
        case inputProcessingBranch = "input_processing_branch"
        case oppoCameraTail = "oppo_camera_tail"
        case tmapFormat = "tmap_format"
        case applePhotographicStyles = "apple_photographic_styles"
        case applePortrait = "apple_portrait"
    }
}

private struct NormalizedPlan: Codable {
    let name: String
    let oppoCompatibility: String
    let requestedInputProcessingBranch: String
    let effectiveInputProcessingBranch: String
    let oppoCameraTail: String
    let tmapFormat: String
    let appleFeatureRoute: String
}

private enum OracleError: Error, CustomStringConvertible {
    case usage
    case invalidValue(caseName: String, field: String, value: String)

    var description: String {
        switch self {
        case .usage:
            return "usage: EnginePlanOracle <conversion_plan_cases.json>"
        case .invalidValue(let caseName, let field, let value):
            return "invalid \(field) '\(value)' in case \(caseName)"
        }
    }
}

private func parse<T: RawRepresentable>(
    _ value: String?,
    default defaultValue: T,
    caseName: String,
    field: String
) throws -> T where T.RawValue == String {
    guard let value else { return defaultValue }
    guard let parsed = T(rawValue: value) else {
        throw OracleError.invalidValue(caseName: caseName, field: field, value: value)
    }
    return parsed
}

private func normalize(_ testCase: PlanCase) throws -> NormalizedPlan {
    var configuration = ConversionConfiguration()
    configuration.oppoCompatibility = try parse(
        testCase.oppoCompatibility,
        default: configuration.oppoCompatibility,
        caseName: testCase.name,
        field: "oppo_compatibility"
    )
    configuration.inputProcessingBranch = try parse(
        testCase.inputProcessingBranch,
        default: configuration.inputProcessingBranch,
        caseName: testCase.name,
        field: "input_processing_branch"
    )
    configuration.oppoCameraTail = try parse(
        testCase.oppoCameraTail,
        default: configuration.oppoCameraTail,
        caseName: testCase.name,
        field: "oppo_camera_tail"
    )
    configuration.tmapFormat = try parse(
        testCase.tmapFormat,
        default: configuration.tmapFormat,
        caseName: testCase.name,
        field: "tmap_format"
    )
    configuration.applePhotographicStyles = testCase.applePhotographicStyles
        ?? configuration.applePhotographicStyles
    configuration.applePortrait = testCase.applePortrait ?? configuration.applePortrait

    let effectiveBranch = resolveEffectiveInputProcessingBranch(
        requested: configuration.inputProcessingBranch,
        oppoCameraTail: configuration.oppoCameraTail,
        tmapFormat: configuration.tmapFormat
    )
    let route = resolveLegacyConversionExecutionRoute(configuration: configuration)

    return NormalizedPlan(
        name: testCase.name,
        oppoCompatibility: configuration.oppoCompatibility.rawValue,
        requestedInputProcessingBranch: configuration.inputProcessingBranch.rawValue,
        effectiveInputProcessingBranch: effectiveBranch.rawValue,
        oppoCameraTail: configuration.oppoCameraTail.rawValue,
        tmapFormat: configuration.tmapFormat.rawValue,
        appleFeatureRoute: route.rawValue
    )
}

private func run() throws {
    let arguments = CommandLine.arguments
    guard arguments.count == 2 else { throw OracleError.usage }
    let data = try Data(contentsOf: URL(fileURLWithPath: arguments[1]))
    let cases = try JSONDecoder().decode([PlanCase].self, from: data)
    let plans = try cases.map(normalize)
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    let output = try encoder.encode(plans)
    FileHandle.standardOutput.write(output)
    FileHandle.standardOutput.write(Data([0x0a]))
}

do {
    try run()
} catch {
    FileHandle.standardError.write(Data("\(error)\n".utf8))
    exit(2)
}
