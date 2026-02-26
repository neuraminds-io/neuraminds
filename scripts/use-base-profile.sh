#!/usr/bin/env bash
set -euo pipefail

profile="${1:-}"
if [[ -z "$profile" ]]; then
  echo "Usage: $0 <sepolia|mainnet>"
  exit 1
fi

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

case "$profile" in
  sepolia)
    cp "$root_dir/.env.base-sepolia.local" "$root_dir/.env"
    cp "$root_dir/web/.env.sepolia.local" "$root_dir/web/.env.local"
    echo "Activated Base Sepolia profile."
    ;;
  mainnet)
    cp "$root_dir/.env.base-mainnet.local" "$root_dir/.env"
    cp "$root_dir/web/.env.mainnet.local" "$root_dir/web/.env.local"
    echo "Activated Base Mainnet profile."
    ;;
  *)
    echo "Unknown profile: $profile"
    echo "Usage: $0 <sepolia|mainnet>"
    exit 1
    ;;
esac
