# Review rubric

Read the current owners and surrounding source before judging the change. Apply only criteria relevant to the diff.

## Correctness and failure behavior

Trace concrete cases through actual callers. Consider empty or malformed input, boundaries, overflow, concurrency, and restart where the change affects them. Inspect the result and required external effects. Check success and required failure behavior. Do not flag a null value or state that cannot reach the code.

For XDRemux, inspect the exact source facts and media-resource relationships affected by the change. Check orientation, dimensions, input/output identity, auxiliary roles, and publication order where relevant. A parser-visible label does not prove that the associated bytes have the stated meaning. Missing required resources or native capability must fail closed.

## Root cause and ownership

Find the first incorrect decision. Determine whether the triggering state is legitimate before adding a guard. Follow callers, callees, types, sibling uses, and persisted state as necessary. Check for swallowed errors, retries, clamps, invented defaults, or discarded resources that hide a broken contract.

Keep policy in the Rust engine and filesystem and lifecycle behavior in runtime. Keep Apple framework operations in the adapter and presentation in the app or CLI owner.

Media, files, helper results, and native inputs remain boundaries when typed internal functions pass them through. Preserve required revalidation of transformed facts and externally observed facts.

## Structure and maintained complexity

Look for a smaller complete sequence of effects, fewer maintained states, and reuse of an existing owner. Assess types and data shapes against actual access patterns.

Before removing an internal API, migrate its actual callers. Preserve intentional external compatibility. Reject an abstraction or second backend that only moves the same complexity elsewhere.

## Verification

Check that tests execute the affected consumer and assert the promised behavior. Expected test values must not come only from the implementation under test. Preserve native lifecycle, wire-byte, exact fixture, and publication assertions when they express the actual contract.

Inspect artifacts produced by delegated or asynchronous work. If the report claims a failing-before and passing-after result, confirm both runs.

Distinguish portable structural checks, native readback, device interaction, and exact-HEAD receipts. An unavailable or failed native check does not establish a successful product test.

## Security and effects

For a security finding, show the input path and the dangerous operation. Where the diff can affect them, inspect these items:

- Shell escaping, output path ownership, and external references.
- Filesystem races, subprocess lifetime, and resource bounds.
- Secret or media exposure.

Verify that failure preserves existing inputs and published outputs. Do not add a general security checklist for unrelated hypothetical risks.
