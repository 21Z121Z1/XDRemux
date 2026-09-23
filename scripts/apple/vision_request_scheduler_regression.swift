enum SchedulerRegressionError: Error {
    case expectedFailure
}

@main
struct VisionRequestSchedulingRegression {
    static func main() throws {
        var observedBatches: [[String]] = []
        try performVisionRequests(["human-attributes", "person", "glasses"]) { requestBatch in
            observedBatches.append(requestBatch)
        }
        precondition(
            observedBatches == [
                ["human-attributes"],
                ["person"],
                ["glasses"],
            ],
            "Vision requests must be delivered as singleton batches in declaration order"
        )

        var callsBeforeFailure = 0
        do {
            try performVisionRequests([1, 2, 3]) { requestBatch in
                callsBeforeFailure += 1
                if requestBatch == [2] {
                    throw SchedulerRegressionError.expectedFailure
                }
            }
            fatalError("the injected Vision request failure must propagate")
        } catch SchedulerRegressionError.expectedFailure {
            precondition(callsBeforeFailure == 2, "request scheduling must stop at the failed request")
        }

        print("PASS Vision request scheduler: singleton batches, ordered, fail-fast")
    }
}
