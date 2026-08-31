# Security Policy

English | [简体中文](SECURITY.zh-CN.md)

## Supported code

Security fixes target the current `main` branch and the latest published release when a release exists. Older commits and unpublished development branches are not maintained as separate security-support lines.

## Report a vulnerability

Use GitHub private vulnerability reporting for security-sensitive findings. Do not open a public issue before the report is triaged when disclosure could enable exploitation.

Include the following information when it is available:

- the affected XDRemux version or commit;
- the operating system and relevant runtime version;
- the input container or metadata type, such as HEIC, JPEG, ISOBMFF, Exif, or TIFF;
- the smallest reproducible input or construction procedure;
- the observed behavior and expected behavior;
- the security impact, including whether the issue can cause memory corruption, arbitrary file access, resource exhaustion, or another boundary violation.

Do not upload private photographs or device data when a synthetic or minimized fixture can reproduce the issue.

The project will use the private report thread to coordinate validation, remediation, and disclosure timing.
