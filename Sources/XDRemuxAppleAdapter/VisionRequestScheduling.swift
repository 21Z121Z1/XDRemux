@inline(__always)
func performVisionRequests<Request>(
    _ requests: [Request],
    perform: ([Request]) throws -> Void
) throws {
    for request in requests {
        try perform([request])
    }
}
