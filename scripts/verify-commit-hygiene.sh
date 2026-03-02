#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

forbidden_a="$(printf '\\153\\141\\155\\151\\171\\157')"
forbidden_b="$(printf '\\143\\154\\141\\165\\144\\145')"
PATTERN="${forbidden_a}|${forbidden_b}"

check_staged=0
check_history_all=0
history_range=""
commit_msg_file=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --staged)
      check_staged=1
      shift
      ;;
    --history-all)
      check_history_all=1
      shift
      ;;
    --history-range)
      if [[ $# -lt 2 ]]; then
        echo "Missing value for --history-range"
        exit 1
      fi
      history_range="$2"
      shift 2
      ;;
    --commit-msg-file)
      if [[ $# -lt 2 ]]; then
        echo "Missing value for --commit-msg-file"
        exit 1
      fi
      commit_msg_file="$2"
      shift 2
      ;;
    *)
      echo "Unknown option: $1"
      exit 1
      ;;
  esac
done

if [[ $check_staged -eq 0 && $check_history_all -eq 0 && -z "$history_range" && -z "$commit_msg_file" ]]; then
  echo "No checks requested. Use one of: --staged | --history-all | --history-range <range> | --commit-msg-file <file>"
  exit 1
fi

fail=0

if [[ $check_staged -eq 1 ]]; then
  staged_patch="$(git diff --cached --no-color || true)"
  if [[ -n "$staged_patch" ]] && echo "$staged_patch" | grep -E -i -n "$PATTERN" >/tmp/commit_hygiene_staged_hits.txt; then
    echo "Forbidden term found in staged diff."
    sed -n '1,80p' /tmp/commit_hygiene_staged_hits.txt
    fail=1
  fi
fi

if [[ -n "$commit_msg_file" ]]; then
  if [[ ! -f "$commit_msg_file" ]]; then
    echo "Commit message file not found: $commit_msg_file"
    exit 1
  fi
  if grep -E -i -n "$PATTERN" "$commit_msg_file" >/tmp/commit_hygiene_msg_hits.txt; then
    echo "Forbidden term found in commit message."
    sed -n '1,80p' /tmp/commit_hygiene_msg_hits.txt
    fail=1
  fi
fi

if [[ $check_history_all -eq 1 ]]; then
  if git log --all --regexp-ignore-case --extended-regexp --grep="$PATTERN" --pretty=format:'%h %s' >/tmp/commit_hygiene_history_hits.txt; then
    if [[ -s /tmp/commit_hygiene_history_hits.txt ]]; then
      echo "Forbidden term found in repository history."
      sed -n '1,120p' /tmp/commit_hygiene_history_hits.txt
      fail=1
    fi
  fi
fi

if [[ -n "$history_range" ]]; then
  if git rev-list "$history_range" >/dev/null 2>&1; then
    if git log --regexp-ignore-case --extended-regexp --grep="$PATTERN" --pretty=format:'%h %s' "$history_range" >/tmp/commit_hygiene_range_hits.txt; then
      if [[ -s /tmp/commit_hygiene_range_hits.txt ]]; then
        echo "Forbidden term found in commit history range: $history_range"
        sed -n '1,120p' /tmp/commit_hygiene_range_hits.txt
        fail=1
      fi
    fi
  fi
fi

if [[ $fail -ne 0 ]]; then
  exit 1
fi

echo "Commit hygiene check passed."
