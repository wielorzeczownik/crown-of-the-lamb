# Security

Thanks for taking the time to look. Please report vulnerabilities privately through [GitHub Security Advisories](https://github.com/wielorzeczownik/crown-of-the-lamb/security/advisories/new) instead of a public issue, so there's a chance to fix them first. You'll be credited in the release notes unless you'd rather not be. Only the latest release gets fixes.

## In scope

The firmware runs its own open WiFi access point with a captive portal, so anyone in WiFi range can reach it. The scope is:

- the access point and captive portal (DHCP, DNS, HTTP)
- the configuration API behind the portal
- the OTA firmware update endpoint
- configuration stored in flash

Problems in upstream crates (esp-hal, embassy, picoserve and others) belong to those projects. If a vulnerable version is pinned here, please still let us know so it can be bumped.

## Running it safely

- Use the prop where you control who is in WiFi range. The portal has no authentication by design.
- Only flash firmware from trusted sources, and check it against the `SHA256SUMS` published with each release.
