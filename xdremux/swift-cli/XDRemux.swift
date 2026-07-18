#!/usr/bin/env swift

import Foundation
import Darwin

let sourceURL = URL(fileURLWithPath: #filePath).standardizedFileURL
let packageRoot = sourceURL
    .deletingLastPathComponent()
    .deletingLastPathComponent()
    .deletingLastPathComponent()

let buildProcess = Process()
buildProcess.executableURL = URL(fileURLWithPath: "/usr/bin/env")
buildProcess.arguments = [
    "swift",
    "build",
    "--quiet",
    "--package-path",
    packageRoot.path,
    "--product",
    "xdremux",
]
buildProcess.standardOutput = FileHandle.nullDevice
buildProcess.standardError = FileHandle.nullDevice

do {
    try buildProcess.run()
    buildProcess.waitUntilExit()
    guard buildProcess.terminationStatus == 0 else {
        throw NSError(
            domain: "XDRemuxCompatibilityEntry",
            code: Int(buildProcess.terminationStatus),
            userInfo: [NSLocalizedDescriptionKey: "Swift package build failed"]
        )
    }

    let executableURL = packageRoot
        .appendingPathComponent(".build")
        .appendingPathComponent("debug")
        .appendingPathComponent("xdremux")
    let process = Process()
    process.executableURL = executableURL
    process.arguments = Array(CommandLine.arguments.dropFirst())
    try process.run()
    process.waitUntilExit()
    exit(process.terminationStatus)
} catch {
    FileHandle.standardError.write(Data("error: unable to launch XDRemux package CLI: \(error)\n".utf8))
    exit(1)
}
