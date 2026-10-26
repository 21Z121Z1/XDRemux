# Security policy

English | [简体中文](SECURITY.zh-CN.md)

## Supported code

Security fixes apply to the current `main` branch. When a published release exists, fixes also apply to the latest release. Older commits and unpublished development branches do not have separate security maintenance.

## Report a vulnerability privately

If disclosure could enable exploitation, do not open a public issue before the report has been triaged. Use GitHub private vulnerability reporting for security-sensitive findings.

When available, include:

- The affected XDRemux version or commit.
- The operating system and relevant runtime version.
- The input container or metadata type, such as HEIC, JPEG, ISOBMFF, Exif, or TIFF.
- The smallest input or construction procedure that reproduces the issue.
- The observed behavior and the expected behavior.
- The security impact, such as memory corruption, arbitrary file access, resource exhaustion, or another boundary violation.

If a synthetic or minimized fixture reproduces the issue, do not upload private photographs or device data.

The project uses the private report thread to coordinate validation, fixes, and disclosure timing.
