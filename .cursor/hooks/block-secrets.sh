#!/usr/bin/env bash
# Cursor beforeShellExecution: block git commit when staged content has secrets.
set -euo pipefail

input="$(cat)"
command="$(printf '%s' "$input" | jq -r '.command // empty')"

allow() {
  printf '%s\n' '{"permission":"allow"}'
  exit 0
}

deny() {
  local msg="$1"
  jq -n --arg u "$msg" --arg a "$msg" \
    '{permission:"deny", user_message:$u, agent_message:$a}'
  exit 0
}

# Only gate commits (matcher already filters; keep script safe if reused).
if [[ ! "$command" =~ git[[:space:]]+commit ]]; then
  allow
fi

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

# Cursor/agent shells often lack an activated mise. Prefer project tools.
if ! command -v mise >/dev/null 2>&1; then
  for candidate in "${HOME}/.local/bin/mise" /usr/local/bin/mise /opt/homebrew/bin/mise; do
    if [[ -x "$candidate" ]]; then
      export PATH="$(dirname "$candidate"):${PATH}"
      break
    fi
  done
fi

run_betterleaks() {
  if command -v mise >/dev/null 2>&1; then
    mise exec -- betterleaks "$@"
  elif command -v betterleaks >/dev/null 2>&1; then
    betterleaks "$@"
  else
    return 127
  fi
}

# Block known sensitive paths even if the scanner is missing.
staged="$(git diff --cached --name-only --diff-filter=ACM 2>/dev/null || true)"
sensitive_re='(^|/)\.env($|\.)|\.pem$|\.p12$|\.p8$|\.jks$|\.keystore$|key\.properties$|\.mobileprovision$|\.provisionprofile$|(^|/)credentials\.json$|AuthKey_.*\.p8$|google-services\.json$|GoogleService-Info\.plist$'
if printf '%s\n' "$staged" | grep -Eiq "$sensitive_re"; then
  deny "Commit blocked: staged path looks like a secret/signing/env file. Unstage it and keep secrets out of git."
fi

set +e
scan_out="$(run_betterleaks git --staged --redact --verbose --no-banner --no-color 2>&1)"
scan_ec=$?
set -e

if [[ $scan_ec -eq 127 ]]; then
  deny "Commit blocked: betterleaks is not available. Install with: curl -fsSL https://mise.run | sh && mise install (see mise.toml)."
fi

if [[ $scan_ec -ne 0 ]]; then
  summary="$(printf '%s\n' "$scan_out" | tail -n 40)"
  deny "Commit blocked: betterleaks found potential secrets in staged files.

${summary}

Unstage/remove the secret, rotate it if it was real, then commit again."
fi

allow
