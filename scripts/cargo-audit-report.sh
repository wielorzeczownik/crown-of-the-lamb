#!/usr/bin/env bash
set -uo pipefail

errors=$(mktemp)
audit=$(cargo audit --json "$@" 2>"$errors")

if ! jq -e '.vulnerabilities' <<<"$audit" >/dev/null 2>&1; then
  printf '## cargo audit Report\n\n⚠️ cargo audit could not run: %s\n' "$(grep -v '^\s*$' "$errors" | tail -n 1 | sed 's/^[[:space:]]*//')"
  exit 1
fi

jq -r '
  def roundup: (. * 10 | ceil) / 10;
  def cvss:
    (split("/") | map(select(contains(":")) | split(":") | {(.[0]): .[1]}) | add) as $m
    | ({H: 0.56, L: 0.22, N: 0}) as $cia
    | (1 - (1 - $cia[$m.C]) * (1 - $cia[$m.I]) * (1 - $cia[$m.A])) as $iss
    | (if $m.S == "C" then 7.52 * ($iss - 0.029) - 3.25 * pow($iss - 0.02; 15) else 6.42 * $iss end) as $impact
    | (8.22 * {N: 0.85, A: 0.62, L: 0.55, P: 0.2}[$m.AV] * {L: 0.77, H: 0.44}[$m.AC]
       * (if $m.S == "C" then {N: 0.85, L: 0.68, H: 0.5} else {N: 0.85, L: 0.62, H: 0.27} end)[$m.PR]
       * {N: 0.85, R: 0.62}[$m.UI]) as $exploit
    | if $impact <= 0 then 0
      elif $m.S == "C" then [1.08 * ($impact + $exploit), 10] | min | roundup
      else [$impact + $exploit, 10] | min | roundup end;
  def severity:
    if . == null then "n/a"
    else cvss as $s
      | (if $s >= 9 then "critical" elif $s >= 7 then "high" elif $s >= 4 then "moderate" elif $s > 0 then "low" else "none" end) end;
  def icon: {critical: "🔴", high: "🟠", moderate: "🟡", low: "⚪"}[.] // "";
  def order: {critical: 0, high: 1, moderate: 2, low: 3}[.] // 4;
  def link: if .url then "[\(.id): \(.title)](\(.url))" else "\(.id): \(.title)" end;
  [.vulnerabilities.list[] | . + {sev: (.advisory.cvss | severity)}] as $vulns
  | [.warnings | to_entries[] | .key as $kind | .value[] | . + {kind: $kind}] as $warnings
  | "## cargo audit Report", "",
    "### Summary", "",
    "- **Dependencies:** \(.lockfile["dependency-count"]) checked",
    (if ($vulns | length) == 0 then "- **Advisories:** ✅ none"
     else "- **Advisories:** ❌ \($vulns | length) total · " + ($vulns | group_by(.sev) | sort_by(.[0].sev | order) | map("\(.[0].sev | icon) \(length) \(.[0].sev)") | join(" · ")) end),
    (if ($warnings | length) > 0 then "- **Warnings:** ⚠️ \($warnings | length) total · " + ($warnings | group_by(.kind) | map("\(length) \(.[0].kind)") | join(" · ")) else empty end),
    (if ($vulns | length) > 0 then
       "", "### Advisories", "",
       "| Package | Severity | Advisory | Fix |",
       "| --- | --- | --- | --- |",
       ($vulns | sort_by(.sev | order) | .[]
        | "| `\(.package.name)` \(.package.version) | \(.sev | icon) \(.sev) | \(.advisory | link) | \(if (.versions.patched | length) > 0 then (.versions.patched | map("`\(.)`") | join(", ")) else "none" end) |")
     else empty end),
    (if ($warnings | length) > 0 then
       "", "### Warnings", "",
       "| Package | Kind | Advisory |",
       "| --- | --- | --- |",
       ($warnings | sort_by(.kind, .package.name) | .[]
        | "| `\(.package.name)` \(.package.version) | \(.kind) | \(if .advisory then (.advisory | link) else "n/a" end) |")
     else empty end)
' <<<"$audit"

[[ "$(jq '.vulnerabilities.count' <<<"$audit")" == 0 ]]
