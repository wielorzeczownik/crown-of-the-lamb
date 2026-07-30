#!/usr/bin/env bash
set -uo pipefail

: "${GITHUB_OUTPUT:?GITHUB_OUTPUT is required}"

report="${REPORT_FILE:-cargo-audit-report.txt}"

emit() {
  echo "$1=$2" >>"$GITHUB_OUTPUT"
}

emit_report() {
  {
    echo 'report<<CARGO_AUDIT_EOF'
    if [[ -s "$report" ]]; then
      cat "$report"
    else
      echo 'cargo audit produced no output, see the workflow logs.'
    fi
    echo 'CARGO_AUDIT_EOF'
  } >>"$GITHUB_OUTPUT"
}

changed=false
unresolved=false

if ! cargo audit --color never 2>&1 | tee "$report"; then
  echo "Advisories found in Cargo.lock, attempting cargo audit fix"
  cargo audit fix || echo "cargo audit fix could not resolve everything"

  if ! git diff --quiet -- Cargo.lock; then
    changed=true
  fi

  unresolved=true
  if cargo audit --color never 2>&1 | tee "$report"; then
    unresolved=false
  fi
fi

emit changed "$changed"
emit unresolved "$unresolved"
emit_report

echo "Cargo.lock changed: $changed, advisories unresolved: $unresolved"
