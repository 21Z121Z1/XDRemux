# Licensing

XDRemux's original software code is licensed under the **GNU General Public License, version 3 only (GPL-3.0-only)**. See [LICENSE](LICENSE) for the full terms.

## Scope

This grant covers the original Rust, Swift, Objective-C, Python, build, and research source code for which `21Z121Z1` holds copyright. It also covers project-authored documentation unless a file identifies a different license.

The following materials **are not licensed under GPL-3.0-only**:

- `fixtures/`: original photographic and video media are governed by [the research-fixture permission](fixtures/LICENSE.md). Technical facts learned from the files are not automatically licensed as photographic works.
- `research/oppo_styles/models/*.mlpackage/`: Core ML model structures and weights are separately copyrighted. See [model licensing](research/oppo_styles/models/LICENSING.md). Project-authored model cards and training scripts remain within the software/documentation grant.
- `.agents/skills/`: adapted third-party skills retain their own MIT notices and upstream attribution.
- Third-party libraries and other components, including [LibLivePhoto](https://github.com/21Z121Z1/LibLivePhoto) (Apache-2.0), x265, libheif, libde265, and libwebp, retain their respective original licenses. Their presence in a GPL-covered distribution does not change their underlying licenses.

The software license does not grant patent rights that contributors do not possess, HEVC patent-pool licenses, Apple SDK rights, rights of people depicted in media, or permission to use third-party trademarks.

## Earlier versions: additional GPLv3 grant

**Effective 29 October 2026**, `21Z121Z1`, as copyright holder, additionally offers the original XDRemux software code under `GPL-3.0-only` for the complete source snapshots identified by these existing release tags:

- [v1.0](https://github.com/21Z121Z1/XDRemux/tree/v1.0)
- [v1.1](https://github.com/21Z121Z1/XDRemux/tree/v1.1)
- [v1.2](https://github.com/21Z121Z1/XDRemux/tree/v1.2)
- [v1.3](https://github.com/21Z121Z1/XDRemux/tree/v1.3)
- [v1.3.1](https://github.com/21Z121Z1/XDRemux/tree/v1.3.1)
- [v1.4](https://github.com/21Z121Z1/XDRemux/tree/v1.4)

This is a **new, additional permission** to use those copyright-controlled works under GPLv3. It does not claim that their historical archives already included a GPL notice, retroactively impose obligations on past conduct, withdraw any independently valid permissions, or relicense code owned by others. Earlier statements describing the project as "for technical research" are usage cautions, not restrictions on this new GPL grant. The fixture and model exclusions above apply to those historical snapshots as well wherever the corresponding kinds of materials are present.

Do not rewrite historical tags to suggest that notices were present when they were originally released. When distributing a binary from any release, supply or clearly identify its *matching* Corresponding Source and preserve third-party license notices.

## Distribution

For the current macOS package, `manifest.json` identifies the exact source commit. `CORRESPONDING_SOURCE.md` gives its source URL and build entry points. Native dependency source archives and license notices are included in the package; Rust dependency licenses are inventoried from the resolved Cargo metadata. A binary or package test alone does not certify complete legal compliance.

Copyright © 2026 21Z121Z1, for the works owned by this copyright holder.
