#!/usr/bin/env bash
set -uo pipefail

: "${GITHUB_OUTPUT:?GITHUB_OUTPUT is required}"

portal_dir="portal"
cargo_report="${CARGO_REPORT_FILE:-cargo-audit-report.txt}"
npm_report="${NPM_REPORT_FILE:-npm-audit-report.txt}"

emit() {
  echo "$1=$2" >>"$GITHUB_OUTPUT"
}

emit_report() {
  {
    echo 'report<<AUDIT_REPORT_EOF'
    echo '## cargo audit'
    echo
    if [[ -s "$cargo_report" ]]; then
      cat "$cargo_report"
    else
      echo 'cargo audit produced no output, see the workflow logs.'
    fi
    echo
    echo '## npm audit (portal)'
    echo
    if [[ -s "$npm_report" ]]; then
      cat "$npm_report"
    else
      echo 'npm audit produced no output, see the workflow logs.'
    fi
    echo 'AUDIT_REPORT_EOF'
  } >>"$GITHUB_OUTPUT"
}

cargo_changed=false
cargo_unresolved=false

if ! cargo audit --color never 2>&1 | tee "$cargo_report"; then
  echo "Advisories found in Cargo.lock, attempting cargo audit fix"
  cargo audit fix || echo "cargo audit fix could not resolve everything"

  if ! git diff --quiet -- Cargo.lock; then
    cargo_changed=true
  fi

  cargo_unresolved=true
  if cargo audit --color never 2>&1 | tee "$cargo_report"; then
    cargo_unresolved=false
  fi
fi

npm_changed=false
npm_unresolved=false
npm_production=false

npm_report_path="${PWD}/${npm_report}"

if ! (cd "$portal_dir" && npm audit) >"$npm_report_path" 2>&1; then
  if ! (cd "$portal_dir" && npm audit --omit=dev) >/dev/null 2>&1; then
    npm_production=true
  fi

  echo "Advisories found in portal/package-lock.json, attempting npm audit fix"
  (cd "$portal_dir" && npm audit fix) || echo "npm audit fix could not resolve everything"

  if ! git diff --quiet -- "${portal_dir}/package-lock.json"; then
    npm_changed=true
  fi

  npm_unresolved=true
  if (cd "$portal_dir" && npm audit) >"$npm_report_path" 2>&1; then
    npm_unresolved=false
  fi
fi

cat "$npm_report_path"

unresolved=false
if [[ "$cargo_unresolved" == "true" || "$npm_unresolved" == "true" ]]; then
  unresolved=true
fi

emit cargo_changed "$cargo_changed"
emit npm_changed "$npm_changed"
emit npm_production "$npm_production"
emit unresolved "$unresolved"
emit_report

echo "Cargo.lock changed: $cargo_changed, npm lockfile changed: $npm_changed"
echo "npm advisories reach production: $npm_production, unresolved: $unresolved"
