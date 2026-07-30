---
title: Unresolved security advisories
labels: security-advisory
---

`cargo audit` reports advisories that `cargo audit fix` could not resolve
automatically, so `Cargo.lock` cannot be updated without a manual decision.

## Audit output

```text
{{ env.AUDIT_REPORT }}
```
