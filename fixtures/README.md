# Fixtures

This directory documents test fixture policy.

Do not commit large real-world `.heic`, `.apk`, `.zip`, or private sample dumps here. Keep large or sensitive samples outside git, in release artifacts, Git LFS, or a private sample store.

Allowed fixture types:

- Tiny synthetic JSON, plist, or text fixtures.
- Minimal binary snippets that are not private user photos and are small enough for normal git review.
- Hash manifests and sample inventories that describe external test files without embedding them.

Recommended manifest fields for external samples:

```text
sample_id:
source_device:
format_family: LHDR|UHDR|ISO-gain-map|plain-HEIC
sha256:
expected_behavior:
notes:
```

Validation scripts should accept a local sample directory path rather than assuming samples are checked into the repository.

## CI fixture archive

The macOS GitHub Actions workflow optionally downloads a private archive from
the `XDREMUX_FIXTURE_ARCHIVE_URL` repository secret. The extracted archive must
contain:

```text
standard.heic
oppo.heic
apple-styles.heic
apple-portrait.heic
```

`standard.heic` and `oppo.heic` must be genuine ProXDR inputs whose expected
outputs are respectively `444f` and OPPO-compatible `420f`. The Apple samples
must contain the real source metadata required by their feature paths. When the
secret is absent, CI reports these checks as `Not configured`; it does not
substitute synthetic private data.
