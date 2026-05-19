# Security Policy

## Supported Versions

Only the latest released version of susshi receives security fixes.

| Version | Supported |
|---------|-----------|
| latest  | ✓         |
| older   | ✗         |

## Reporting a Vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

Report vulnerabilities privately via [GitHub Security Advisories](https://github.com/yatoub/susshi/security/advisories/new).

Include:
- A description of the vulnerability and its potential impact.
- Steps to reproduce or a proof-of-concept (sanitized — no real credentials).
- Your suggested fix, if any.

You will receive an acknowledgement within **72 hours** and a status update within **7 days**.

## Scope

Security issues in scope:
- Credential or key exposure via config parsing or TUI output.
- Command injection through config values or user input.
- Unintended network access or data exfiltration.
- Unsafe handling of SSH agent sockets or PTY sessions.

Out of scope:
- Vulnerabilities requiring physical access to the machine.
- Issues in third-party dependencies (report those upstream; we track them via `cargo audit`).

## Dependency Auditing

This project runs `cargo audit` in CI on every push and pull request to detect known vulnerabilities in dependencies.
