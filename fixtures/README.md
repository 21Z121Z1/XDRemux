# Real Motion Photo fixtures

This directory contains the real-world Motion Photo inputs used by the strict Swift and pure-Python CI gates.

The 14 image files are intentionally committed byte-for-byte as supplied. Their EXIF, GPS, capture timestamps, vendor metadata, embedded video resources, gain maps, and other payloads are not sanitized or rewritten because the tests validate exact file identity and container geometry.

`SHA256SUMS` is the canonical identity manifest. CI and the conversion tests reject fixtures whose bytes differ from these hashes.

These fixtures cover OPPO/ColorOS, Xiaomi, Samsung HEIF/JPEG Motion Photo, and vivo inputs. Generated HEIC/MOV outputs remain temporary CI artifacts and are not committed here.
