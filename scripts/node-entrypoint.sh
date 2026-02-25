#!/usr/bin/env bash
set -euo pipefail

NODE_INDEX="${NODE_INDEX:?}"
COUNT="${COUNT:-4}"
KEY_DIR="${KEY_DIR:-/keys}"

BIND="0.0.0.0:800${NODE_INDEX}"
ADVERTISE="172.30.0.1${NODE_INDEX}:800${NODE_INDEX}"   # 11..14 mapping
KEY_FILE="${KEY_DIR}/node${NODE_INDEX}.secret"

/app/scripts/wait-for-keys.sh

PEER_ARGS=()
for i in $(seq 1 "${COUNT}"); do
  if [ "${i}" = "${NODE_INDEX}" ]; then
    continue
  fi
  PUB="$(tr -d '\r\n' < "${KEY_DIR}/node${i}.public")"
  PEER_IP="172.30.0.1${i}"
  PEER_ARGS+=( "--peer" "${PUB}@${PEER_IP}:800${i}" )
done

echo "node${NODE_INDEX} bind=${BIND} advertise=${ADVERTISE} peers: ${PEER_ARGS[*]}"

exec /usr/local/bin/tashi-demo-node \
  --node-id "node${NODE_INDEX}" \
  --bind "${BIND}" \
  --advertise "${ADVERTISE}" \
  --key-file "${KEY_FILE}" \
  "${PEER_ARGS[@]}" \
  --inc-every-secs 5 \
  --inc-amount 1
