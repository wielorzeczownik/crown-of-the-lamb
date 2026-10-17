#!/usr/bin/env bash
set -uo pipefail

audit=$(npm audit --json 2>/dev/null)

jq -r '
  def icon: {critical: "🔴", high: "🟠", moderate: "🟡", low: "⚪", info: "🔵"}[.] // "";
  def order: {critical: 0, high: 1, moderate: 2, low: 3, info: 4}[.] // 5;
  "## npm audit Report", "",
  if .metadata == null then
    "⚠️ npm audit could not run: \(.error.summary // "unknown error")"
  else
    .metadata.vulnerabilities as $v
    | "### Summary", "",
      "- **Dependencies:** \(.metadata.dependencies.total) checked",
      (if $v.total == 0 then "- **Advisories:** ✅ none"
       else "- **Advisories:** ❌ \($v.total) total · " + ([ "critical", "high", "moderate", "low", "info" ] | map(select($v[.] > 0) | "\(icon) \($v[.]) \(.)") | join(" · ")) end),
      (if $v.total > 0 then
         "", "### Advisories", "",
         "| Package | Severity | Advisory | Fix |",
         "| --- | --- | --- | --- |",
         ([.vulnerabilities[]] | sort_by(.severity | order) | .[]
          | ([.via[] | objects] | first) as $a
          | "| `\(.name)`\(if .isDirect then "" else " (transitive)" end) | \(.severity | icon) \(.severity) | \(if $a then "[\($a.title)](\($a.url))" else "via " + ([.via[] | strings] | map("`\(.)`") | join(", ")) end) | \(if .fixAvailable == true then "`npm audit fix`" elif (.fixAvailable | type) == "object" then "`\(.fixAvailable.name)@\(.fixAvailable.version)`\(if .fixAvailable.isSemVerMajor then " ⚠️ major" else "" end)" else "none" end) |")
       else empty end)
  end
' <<<"$audit"

[[ "$(jq '.metadata.vulnerabilities.total // -1' <<<"$audit")" == 0 ]]
