---
title: Unresolved security advisories
labels: security-advisory
---

`cargo audit` or `npm audit` reports advisories that the matching `fix`
subcommand could not resolve automatically, so the lockfile cannot be updated
without a manual decision.

## Audit output

```text
{{ env.AUDIT_REPORT }}
```
