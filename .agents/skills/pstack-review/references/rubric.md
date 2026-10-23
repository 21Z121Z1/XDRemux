# Review rubric

Apply criteria relevant to the diff. Read current owners and surrounding source before judging it.

## Correctness and failure behavior

Trace concrete empty, malformed, boundary, overflow, concurrency, or restart cases through actual callers. Inspect the result and required external effects. Check both success and necessary failures. Do not flag a hypothetical null or state that cannot reach this code.

For XDRemux, check exact source facts and media-resource relationships, orientation and dimensions, input/output identity, auxiliary roles, and publication order where the change affects them. A parser-visible label does not prove that the associated bytes have the advertised semantics. Missing required resources or native capability must fail closed.

## Root cause and ownership

Find the first incorrect decision rather than adding a symptom guard. Check whether the triggering state is legitimate. Follow callers, callees, types, sibling uses, and persisted state as necessary. Question swallowed errors, retries, clamps, invented defaults, or discarded resources that hide a violated contract.

Keep policy in the Rust engine, filesystem/lifecycle behavior in runtime, Apple framework operations in the adapter, and presentation in the app/CLI owner. Media/file/helper/native inputs remain boundaries even when passed through typed internal functions. Do not remove required revalidation of transformed or externally observed facts.

## Structure and maintained complexity

Look for a smaller complete effect path, fewer maintained states, and reuse of an existing owner. Assess types and data shapes against real access patterns. Migrate actual callers before removing an internal API; preserve intentional external compatibility. Reject an abstraction or second backend that merely relocates the same complexity.

## Verification

Check that tests execute the affected consumer and assert the promised behavior. A test expected value must not simply be computed by the implementation under test. Keep native lifecycle, wire-byte, exact fixture, and publication assertions when those are the actual contract.

Inspect actual artifacts from delegated or asynchronous work. Confirm the failing-before evidence and passing-after result when claimed. Distinguish portable structure, native readback, device interaction, and exact-HEAD receipts. An unavailable or failed native check is not a successful product test.

## Security and effects

For a security finding, show the input path and dangerous operation. Inspect shell escaping, output path ownership, external references, filesystem races, subprocess lifetime, resource bounds, and secret/media exposure only where the diff can affect them. Verify failure preserves existing inputs and published outputs. Do not add a generic security checklist for unrelated hypothetical risks.
