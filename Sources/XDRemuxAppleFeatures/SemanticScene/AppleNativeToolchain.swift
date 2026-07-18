import Foundation
import XDRemuxCore

package enum AppleNativeToolchain {
    struct Result {
        let status: Int32
        let stdout: Data
        let stderr: Data
    }

    private static let cancellationKey = "com.proxdr.XDRemux.AppleNativeToolchain.cancellation"

    static func withCancellation<T>(
        _ cancellation: ConversionCancellation?,
        operation: () throws -> T
    ) rethrows -> T {
        let dictionary = Thread.current.threadDictionary
        let previous = dictionary[cancellationKey]
        if let cancellation {
            dictionary[cancellationKey] = cancellation
        } else {
            dictionary.removeObject(forKey: cancellationKey)
        }
        defer {
            if let previous {
                dictionary[cancellationKey] = previous
            } else {
                dictionary.removeObject(forKey: cancellationKey)
            }
        }
        return try operation()
    }

    static func semanticExecutable() throws -> URL {
        try executable(named: "XDRemuxSemanticHelper")
    }

    static func hevcEncoderExecutable() throws -> URL {
        try executable(named: "XDRemuxHEVCEncoderHelper")
    }

    static func stylePropertiesProbeExecutable() throws -> URL {
        try executable(named: "XDRemuxStyleValidationHelper")
    }

    static func run(
        _ executableURL: URL,
        arguments: [String],
        timeout: TimeInterval = 600,
        cancellation: ConversionCancellation? = nil
    ) throws -> Result {
        let effectiveCancellation = cancellation
            ?? Thread.current.threadDictionary[cancellationKey] as? ConversionCancellation
        try effectiveCancellation?.checkCancellation()
        let process = Process()
        process.executableURL = executableURL
        process.arguments = arguments
        let output = Pipe()
        let errors = Pipe()
        process.standardOutput = output
        process.standardError = errors
        do {
            try process.run()
        } catch {
            throw CLIError.invalidContainer(
                "cannot launch Apple feature helper \(executableURL.lastPathComponent): \(error)"
            )
        }
        let stdout = DataReader(output.fileHandleForReading)
        let stderr = DataReader(errors.fileHandleForReading)
        let deadline = Date().addingTimeInterval(timeout)
        while process.isRunning {
            if effectiveCancellation?.isCancelled == true {
                process.terminate()
                process.waitUntilExit()
                stdout.wait()
                stderr.wait()
                throw CancellationError()
            }
            if Date() >= deadline {
                process.terminate()
                process.waitUntilExit()
                stdout.wait()
                stderr.wait()
                throw CLIError.appleFeatureRuntimeUnavailable(
                    "helper \(executableURL.lastPathComponent) timed out after \(Int(timeout)) seconds"
                )
            }
            Thread.sleep(forTimeInterval: 0.02)
        }
        stdout.wait()
        stderr.wait()
        return Result(
            status: process.terminationStatus,
            stdout: stdout.data,
            stderr: stderr.data
        )
    }

    private static func executable(named name: String) throws -> URL {
        let fileManager = FileManager.default
        var candidates: [URL] = []
        if let override = ProcessInfo.processInfo.environment["XDREMUX_HELPER_DIRECTORY"] {
            candidates.append(URL(fileURLWithPath: override, isDirectory: true).appendingPathComponent(name))
        }
        if Bundle.main.bundleURL.pathExtension == "app" {
            candidates.append(
                Bundle.main.bundleURL
                    .appendingPathComponent("Contents/Helpers", isDirectory: true)
                    .appendingPathComponent(name)
            )
        }
        if let executableDirectory = Bundle.main.executableURL?.deletingLastPathComponent() {
            candidates.append(executableDirectory.appendingPathComponent(name))
        }
        if let match = candidates.first(where: { fileManager.isExecutableFile(atPath: $0.path) }) {
            return match
        }
        throw CLIError.appleFeatureRuntimeUnavailable(
            "missing prebuilt helper \(name); searched \(candidates.map(\.path).joined(separator: ", "))"
        )
    }
}

private final class DataReader: @unchecked Sendable {
    private let group = DispatchGroup()
    private let lock = NSLock()
    private var storage = Data()

    init(_ handle: FileHandle) {
        group.enter()
        DispatchQueue.global(qos: .utility).async { [self] in
            let value = handle.readDataToEndOfFile()
            lock.lock()
            storage = value
            lock.unlock()
            group.leave()
        }
    }

    func wait() {
        group.wait()
    }

    var data: Data {
        lock.lock()
        defer { lock.unlock() }
        return storage
    }
}
