# Texture image-processing flags — 2026-11-06

## Scope

This change makes `assemble_texture_styles_heif` update the active Exif
MakerNote `ImageProcessingFlags` tag `0x0019` while appending Texture
metadata. It ORs `0x00200000` for Styles and `0x00002000` for Texture Style.

Existing flags remain set. If tag 25 is absent, the writer rebuilds the
MakerNote from its current entries, relocating out-of-line values and retaining
the other entries. The writer rejects duplicate tag 25, a non-32-bit tag 25,
an unknown TIFF field type, or a non-zero next-IFD pointer. It verifies the
required bits in the assembled output before returning it.

## Evidence

The focused regression uses a complete synthetic HEIF graph with an Exif item
and an Apple MakerNote. It covers both an existing tag 25 and a note missing
tag 25, and checks that the tag84 value remains present.

The full `xdremux-heif` crate test suite checks that source HDR resources and
other item payloads survive the same assembly. A separate functional probe ran
the existing Apple Styles consumer script on a public ProXDR fixture. Its
intermediate active MakerNote contained `tag 25 = 0x00202000`, and the
intermediate final file passed `xdremux validate`.

## Limits

The Apple Styles consumer script does not pass on this host: the native
`texture-readback` step fails both with this change and on unmodified
`b8148ca75184e4a5acb4836f01acfd93abc4edda`. It is therefore not used as a
passing functional or Photos acceptance check for this change.

This record does not claim Photos style/texture activation, save/reopen,
PhotoKit import, Live Photo behavior, or visual acceptance. Those require the
native consumer gate and a target-device round trip.
