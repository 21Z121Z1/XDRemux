import Foundation
import XDTextureNative

struct TexturePrimitiveRequest: Codable {
    let maxDimension: UInt32?
    let contextPath: String?
    let personIndex: UInt32?
    let effect: String?
    let tuning: [String: String]?

    enum CodingKeys: String, CodingKey {
        case maxDimension = "max_dimension"
        case contextPath = "context_path"
        case personIndex = "person_index"
        case effect, tuning
    }
}

// The runtime supplies the profile and coordinates all primitives. No HEIF
// graph, feature routing, fallback, or publication policy enters this bridge.
func runTexturePrimitive(
    operation: String, input: String, output: String,
    configuration: TexturePrimitiveRequest?
) throws {
    let status: Int32 = try input.withCString { inputCString in
        try output.withCString { outputCString in
            switch operation {
            case "texture-prepare":
                guard let limit = configuration?.maxDimension else {
                    throw textureError("Missing analysis dimension")
                }
                return xdremux_texture_prepare(inputCString, outputCString, limit)
            case "texture-infer":
                return xdremux_texture_infer(inputCString, outputCString)
            case "texture-readback":
                return xdremux_texture_readback(inputCString, outputCString)
            case "texture-statistics":
                guard let context = configuration?.contextPath,
                      let person = configuration?.personIndex,
                      let effect = configuration?.effect,
                      ["Mattify", "UnderEyeBrighten", "SkinSmoothStandalone"].contains(effect),
                      let tuning = configuration?.tuning,
                      ["hardware_model", "port_type", "capture_mode", "preset", "capture_type"]
                        .allSatisfy({ tuning[$0] != nil }) else {
                    throw textureError("Missing statistics context, person, effect, or tuning")
                }
                let json = try JSONSerialization.data(withJSONObject: tuning, options: [.sortedKeys])
                let profile = String(decoding: json, as: UTF8.self)
                return context.withCString { contextCString in
                    effect.withCString { effectCString in
                        profile.withCString { tuningCString in
                            xdremux_texture_statistics(
                                inputCString, outputCString, contextCString,
                                person, effectCString, tuningCString
                            )
                        }
                    }
                }
            default:
                throw textureError("Unknown native primitive")
            }
        }
    }
    guard status == 0 else {
        throw textureError("\(operation) failed (status \(status)); native report: \(output)")
    }
}

private func textureError(_ message: String) -> NSError {
    NSError(domain: "XDRemux.TexturePrimitive", code: 1,
            userInfo: [NSLocalizedDescriptionKey: message])
}
