#!/usr/bin/env bash
set -uo pipefail

: "${GITHUB_OUTPUT:?GITHUB_OUTPUT is required}"
: "${NPM_DIR:?NPM_DIR is required}"

report="${REPORT_FILE:-npm-audit-report.txt}"
report_path="${PWD}/${report}"

emit() {
  echo "$1=$2" >>"$GITHUB_OUTPUT"
}

emit_report() {
  {
    echo 'report<<NPM_AUDIT_EOF'
    if [[ -s "$report_path" ]]; then
      cat "$report_path"
    else
      echo 'npm audit produced no output, see the workflow logs.'
    fi
    echo 'NPM_AUDIT_EOF'
  } >>"$GITHUB_OUTPUT"
}

changed=false
unresolved=false
production=false

if ! (cd "$NPM_DIR" && npm audit) >"$report_path" 2>&1; then
  if ! (cd "$NPM_DIR" && npm audit --omit=dev) >/dev/null 2>&1; then
    production=true
  fi

  echo "Advisories found in ${NPM_DIR}/package-lock.json, attempting npm audit fix"
  (cd "$NPM_DIR" && npm audit fix) || echo "npm audit fix could not resolve everything"

  if ! git diff --quiet -- "${NPM_DIR}/package-lock.json"; then
    changed=true
  fi

  unresolved=true
  if (cd "$NPM_DIR" && npm audit) >"$report_path" 2>&1; then
    unresolved=false
  fi
fi

cat "$report_path"

emit changed "$changed"
emit production "$production"
emit unresolved "$unresolved"
emit_report

echo "${NPM_DIR} lockfile changed: $changed, reaches production: $production, unresolved: $unresolved"
