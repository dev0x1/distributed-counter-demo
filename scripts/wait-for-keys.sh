#!/usr/bin/env bash
set -euo pipefail

KEY_DIR="${KEY_DIR:-/keys}"
COUNT="${COUNT:-4}"

for i in $(seq 1 "${COUNT}"); do
  while [ ! -s "${KEY_DIR}/node${i}.secret" ] || [ ! -s "${KEY_DIR}/node${i}.public" ]; do
    echo "waiting for ${KEY_DIR}/node${i}.secret and .public ..."
    sleep 0.5
  done
done

echo "all keys present in ${KEY_DIR}"