# Synthetic high-precision HEVC vectors

These four 64 x 64 ISO Gain Map HEIF files contain generated samples only.
They are test inputs, not photographs or producer interoperability evidence.
The portable preservation test reads them without requiring a high-precision
encoder. This matters for hosts whose x265 build supports only eight bits.
All four vectors are always exercised; an unavailable decoder fails the test.

The files were generated with libheif 1.23.1 and x265 3.5, using these steps:

1. Encode a flat RGB8 base (`[128, 128, 128]`) with
   `LibHeifProvider::encode_primary_heif` and `PrimaryHeifEncodeRequest::live_photo`.
2. Allocate a planar monochrome or RGB444 libheif image at the stated precision.
   For component `c`, fill each pixel with
   `(37*x + 19*y + 137*c + 1) % (1 << bit_depth)`, in native little-endian u16
   plane storage. Set x265 lossless quality and, for RGB, chroma `444`.
3. Extract its primary HEVC resource with `extract_primary_hevc_resource`.
4. Assemble that resource as a single Gain Map tile with
   `assemble_iso_gain_map_heif`, full-range BT.601 compression declarations,
   and a BT.2020 PQ alternate. The base application-space tone-map and XMP
   payloads are generated from this existing float view:
   `[1,1,1,1,4,4,4,1,1,1,0,0,0,0,0,0,1,4]`.
   No static, dynamic, or additional mapping metadata is fabricated.

The preservation test asserts actual decoded low-bit content, declared depth,
channel count, every compressed Gain Map resource, the compressed SDR signature,
and identical decoded Gain Map samples before and after HEIF remux. It also
checks successful eight-bit JPEG export and reconstruction for every vector.
Byte-identical regeneration across encoder versions is not a test requirement.

| File | SHA-256 |
| --- | --- |
| `hevc-10-mono.heic` | `d3890e0546c7ad6db35f2016ba183cfa40e778abeab83fe779704b825421e5a5` |
| `hevc-10-rgb.heic` | `db93a962b9ce60e41758bec499aeb44874e93f2388d00eb2246c26cfbac7b657` |
| `hevc-12-mono.heic` | `8324e13707c5f92f25610194e6ef0315696fd2266fd5e1c7901da8a6bd828d18` |
| `hevc-12-rgb.heic` | `89975c8a6fea1bf6ab7027762a894a58a693245b24dfbba66583590a8cd6b722` |
