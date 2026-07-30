# Security Policy

## Supported versions

Only the latest release receives security fixes.

## Reporting a vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

Report vulnerabilities privately via [GitHub Security Advisories](https://github.com/wielorzeczownik/crown-of-the-lamb/security/advisories/new).

Include as much detail as possible:

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

You will receive a response within **7 days**. If the issue is confirmed, a fix will be released as soon as possible and you will be credited in the release notes (unless you prefer to remain anonymous).

## Scope

This project is firmware for a self-contained ESP32 prop that runs its own WiFi access point and captive portal. The attack surface includes:

- The WiFi access point and captive portal (DHCP, DNS, HTTP endpoints)
- The configuration API exposed by the portal (eye/expression settings, colours, thresholds)
- Configuration persisted to flash

Because the device exposes an open access point by design, anyone within WiFi range can reach the portal. Treat physical/RF proximity as part of the trust boundary.

Issues in upstream crates (esp-hal, embassy, picoserve, etc.) should be reported to their respective projects; if a vulnerable version is pinned here, please still let us know so the dependency can be bumped.

## Security notes for operators

- Deploy the prop only in environments where you control who is in WiFi range.
- Do not expose the portal beyond the device's own access point (there is no authentication by design).
- Only flash firmware built from trusted sources; verify the firmware against the `SHA256SUMS` manifest published with each release.
