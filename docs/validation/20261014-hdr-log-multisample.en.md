# HDR Log thumbnail: 15-photo native comparison

English | [简体中文](20261014-hdr-log-multisample.md)

Replacing the copied SDR thumbnail improves the measured Tone response in the
responsive version-16 research carrier. Direction checks improve from 4/15 to
14/15, with no default-look change. The current version-15 product carrier
remains inactive in this native diagnostic. These are separate results.

## Inputs and controls

The sample set contains all 14 versioned OPPO ProXDR fixtures and the private
blue-curtain photo used in the earlier experiment. There is one Find X6 Pro
photo, six Find X7 Ultra photos, and eight Find X9 Ultra photos. The set covers
LHDR, RGB UHDR, XPAN, high-resolution, Master, and Portrait inputs. It is not a
random sample. The two Master-v2 photos show closely related sunset scenes.

Each source has four outputs: version-15 with an SDR thumbnail, version-15 with
the PR's HDR Log thumbnail, version-16 with an SDR thumbnail, and version-16
with the same PR thumbnail. The PR thumbnail comes from the actual Rust
conversion. The experiment does not reimplement its transform in Python.

Within each carrier pair, only the thumbnail payload and matching HEVC
configuration change. Key1, the complete Styles plist, primary image, Gain Map,
semantic resources, dimensions, and thumbnail transforms remain unchanged.
Exposure key `4` matches between the two source-derived carriers for every
photo. The graph checks passed for all 15 sources. Source and executable hashes
were checked before and after the run.

The native grid uses disabled, neutral, Tone −1, Tone +1, Color −1, Color +1,
Tone −1 / Color +1, and Tone +1 / Color +1. Standard cast, intensity 1, and
maximum dimension 1024 are fixed. All 480 main-grid renders succeeded. Another
14 renders check the remaining direction exception and a one-code repeatability
difference. The platform is macOS 27.0.1 build 26A434; PNG output is Display P3.

## Measured results

An active adjustment requires a control-to-neutral RGB MAE above 0.05/255.
For active carriers, the direction check requires mean linear P3 luminance to
decrease at Tone −1 and increase at Tone +1, with a 0.001 EV tolerance. These
criteria were fixed before rendering. Inactive cases do not count as direction
successes or failures.

| Measurement | Copied SDR thumbnail | HDR Log thumbnail |
| --- | ---: | ---: |
| Active version-16 photos | 15/15 | 15/15 |
| Version-16 Tone direction checks passed | 4/15 | 14/15 |
| Version-16 Tone −1 incorrectly increases mean luminance | 11/15 | 1/15 |
| Version-16 Tone +1 increases mean luminance | 15/15 | 15/15 |
| Neutral pixels identical between each version-16 pair | — | 15/15 |
| Disabled pixels identical between each version-16 pair | — | 15/15 |
| Active version-15 photos | 0/15 | 0/15 |
| Neutral pixels identical between each version-15 pair | — | 15/15 |

Ten direction failures become passes; no passing sample becomes a failure.
The remaining exception is `uhdr-hr-02`: Tone −1 changes from +0.41865 EV to
+0.03115 EV relative to neutral. The latter is about a 2.18% mean-luminance
increase. Three observations, including two repeat runs, give the same result.
It remains a failed direction check; the tolerance was not relaxed.

The next table uses version-16 carriers. EV values compare mean linear P3
luminance with the same file's neutral output. Saturation is the percentage of
SDR PNG pixels with any channel at 255, at Tone +1 / Color +1. Its change is
after minus before, in percentage points (pp).

| Input | Tone −1 before (EV) | Tone −1 after (EV) | Saturation change (pp) |
| --- | ---: | ---: | ---: |
| X6 `lhdr-v1-01` | −0.06747 | −0.54785 | −0.03871 |
| X7 `lhdr-v2-01` | +0.08046 | −0.44260 | −1.28065 |
| X7 `lhdr-v2-02` | +0.28535 | −0.21390 | 0.00000 |
| X7 `lhdr-v2-03` | +0.34370 | −0.03658 | −0.04590 |
| X7 `lhdr-v2-04` | −0.20759 | −0.75992 | −0.00153 |
| X7 `lhdr-v2-05` | −0.10221 | −0.31748 | −0.12080 |
| X7 `lhdr-v2-xpan-01` | +0.22052 | −0.15734 | 0.00000 |
| X9 `uhdr-hr-01` | +0.18886 | −0.46775 | −0.06549 |
| X9 `uhdr-hr-02` | +0.41865 | +0.03115 | −0.25763 |
| X9 `uhdr-master-v1-01` | +0.22714 | −0.09450 | −0.12283 |
| X9 `uhdr-master-v2-01` | +0.45543 | −0.13981 | −3.31497 |
| X9 `uhdr-master-v2-02` | +0.45866 | −0.15523 | −3.23753 |
| X9 `uhdr-portrait-01` | +0.36427 | −0.35469 | −0.06311 |
| X9 `uhdr-portrait-02` | −0.03668 | −0.51874 | −0.02047 |
| X9 private blue-curtain photo | +0.33459 | −0.29061 | −0.37893 |

Combined-endpoint saturation decreases by more than 0.01 pp in 12 photos; the
other three change by at most 0.01 pp. None increases. Mean change per photo is
−0.59657 pp; median change is −0.06549 pp. The two Master-v2 examples change
from 11.08144% to 7.76647% and from 11.68849% to 8.45095%.

At Tone −1, pure-black pixels (all three channels zero) decrease by more than
0.01 pp in 14 photos. The XPAN photo has a smaller decrease. None increases.
Tone +1 remains positive but its mean-luminance increase is smaller with the new
thumbnail in all 15 photos. Saturation changes therefore do not prove recovered
highlight detail at matched brightness, or reduced clipping on an HDR display.

In the version-15 grid, one disabled comparison differs in one channel by one
8-bit code. Its RGB MAE is 0.000000424/255. Repeating the unchanged SDR file
reproduces the same one-code variation. Neutral and all active-control states
are identical within every version-15 pair. This diagnostic still shows no
measurable Tone or Color activation for that carrier.

## Provenance and limits

The PR implementation was measured at `e04c1a9`. The executable SHA-256 values
are recorded here to distinguish the actual artifacts used:

- PR CLI: `4b63b0ec649f92986fa7651dae371eb9049f66835bbe582cc830e4a200460688`.
- Archived version-16 CLI: `2fc2cf158bf1acad164d57e890fb30e8280310292007b0eaa38d41b66ed1fb1b`.
- Native renderer: `476a36a38a88f6860c8dbb6c21133c1c40e1d0dc565a4c257c03e632be51afd7`.

The version-16 producer is an archived research binary, pinned by hash. Its
source reference is `9897b86`; this experiment does not claim a fresh exact-HEAD
build of that archived producer. The protocol, executable snapshots, source
hashes, graph invariants, raw manifests, PNGs, analysis scripts, and CSV results
are retained locally. Private source media and rendered photos are not public
repository artifacts.

The two carrier generations differ in more than a version number. The experiment
does not prove that changing only the version field enables Styles. Within each
generation, however, the thumbnail is the isolated variable.

The result supports the HDR Log thumbnail in a responsive consumer path. It does
not establish visible benefit for the current version-15 product carrier, RAW
recovery, replacement of the complete key1 solver, total conversion speedup, or
Photos import/edit/save/reopen and device HDR display behavior. The smaller
[encoded-resource regression](20261014-hdr-log-thumbnail.en.md) remains the
automated product check.
