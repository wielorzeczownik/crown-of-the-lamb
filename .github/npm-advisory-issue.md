---
title: Unresolved npm advisories
labels: npm-advisory
---

`npm audit` reports advisories in `portal/package-lock.json` that
`npm audit fix` could not resolve automatically, so the lockfile cannot be
updated without a manual decision.

## Audit output

```text
{{ env.NPM_REPORT }}
```
