# Security Policy

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| 0.5.x   | :white_check_mark: |
| < 0.5   | :x:                |

## Reporting a Vulnerability

If you discover a security vulnerability in HydroShot, please report it responsibly.

**Do not open a public issue.** Instead, please email the maintainer directly or use [GitHub's private vulnerability reporting](https://github.com/Real-Fruit-Snacks/HydroShot/security/advisories/new).

### What to include

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### Response timeline

- **Acknowledgment:** Within 48 hours
- **Initial assessment:** Within 1 week
- **Fix and disclosure:** As soon as a patch is ready, coordinated with the reporter

## Scope

The following areas are in scope:

- Command injection (e.g., OCR PowerShell integration)
- Memory safety issues in unsafe Rust code
- Path traversal in file operations
- Credential/secret exposure (e.g., Imgur client IDs)
- Supply chain issues in dependencies or CI

## Past Security Fixes

- **v0.5.3:** Removed hardcoded Imgur client ID, fixed OCR temp file race condition, pinned GitHub Actions to SHA hashes
- **v0.5.2:** Fixed command injection in OCR PowerShell integration, integer overflow in capture buffer sizing
