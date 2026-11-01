#!/usr/bin/env bash
# Archive WaterUptime audit records for real, then retrieve and restore them,
# on a throwaway LOCAL Stellar network (see docs/RETENTION.md).
#
# On testnet nothing archives for months: the network keeps a new entry for at
# least ~7 days and the contract extends its records to 120 days. This script
# instead starts a local quickstart network whose minimum entry lifetime is 100
# ledgers (~1 ledger per second), and deploys a test build with the `short-ttl`
# feature, where the contract's "day" is 4 ledgers. That build differs from a
# release only in its TTL constants. Archival, retrieval and restoration are
# done by the network and the Stellar CLI, not by contract code.
#
# What it checks:
#   1. A closed job's records, its water point, the contract instance and its
#      code all archive.
#   2. While archived, they can still be read through simulation, and
#      `audit-export.sh verify` confirms them unchanged but reports them
#      archived (exit 2).
#   3. `audit-export.sh restore` then `extend` make them live again, and
#      `verify` passes (exit 0) with the history identical to the export.
#   4. After archiving again, a submitted transaction that needs the water
#      point (a new fault report) restores it automatically.
#
# Usage (from the repository root): scripts/local-archival-test.sh
# Takes about 20 minutes, mostly waiting. Needs docker, the Stellar CLI, jq and
# openssl. Creates local-only keys wul-*; nothing touches testnet.
#
# Environment:
#   QUICKSTART_IMAGE  default stellar/quickstart:testing
#   LOCAL_PORT        host port for the local RPC (default 8010)
#   KEEP_NETWORK=1    leave the container running afterwards

set -euo pipefail

IMAGE=${QUICKSTART_IMAGE:-stellar/quickstart:testing}
PORT=${LOCAL_PORT:-8010}
CONTAINER=wu-archival-test
NETWORK=wu-archival-local
PREFIX=wul
WORK=$(mktemp -d)
EXPORT=$WORK/export.json
AUDIT=scripts/audit-export.sh

step() { printf '\n==> %s\n' "$*"; }
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
cleanup() {
  rm -rf "$WORK"
  [[ ${KEEP_NETWORK:-} == 1 ]] || docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
}
trap cleanup EXIT

rand32() { openssl rand -hex 32; }
ledger() { stellar ledger latest --network "$NETWORK" 2>/dev/null | awk '/^Sequence:/ {print $2}'; }
call() {
  local source=$1
  shift
  stellar contract invoke --id "$CONTRACT" --source "$PREFIX-$source" --network "$NETWORK" -- "$@" 2>/dev/null
}
read_fn() { stellar contract invoke --id "$CONTRACT" --source "$PREFIX-reporter" --network "$NETWORK" --send=no -- "$@" 2>/dev/null; }
key_xdr() { jq -cn --arg v "$1" --argjson a "$2" '{vec: [{symbol: $v}, $a]}' | stellar xdr encode --type ScVal; }
# Live-until ledger of a persistent entry; RPC reports 0 for an archived one.
live_until() {
  stellar ledger entry fetch contract-data --contract "$CONTRACT" --key-xdr "$(key_xdr "$1" "$2")" \
    --durability persistent --network "$NETWORK" --output json 2>/dev/null | jq -r '.entries[0].liveUntilLedgerSeq'
}
wait_archived() {
  echo "waiting for $1 to archive (now ledger $(ledger))…"
  until [[ $(live_until "$1" "$2") == 0 ]]; do sleep 10; done
  echo "archived at about ledger $(ledger)"
}
# Run audit-export.sh and check its exit status.
audit() {
  local want=$1 got=0
  shift
  NETWORK=$NETWORK SOURCE=$PREFIX-reporter CONTRACT_ID=$CONTRACT "$AUDIT" "$@" || got=$?
  ((got == want)) || fail "audit-export.sh $1 exited $got, expected $want"
}

step "Starting a local network with a 100-ledger minimum entry lifetime ($IMAGE)"
docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
proto=$(docker run --rm --entrypoint bash "$IMAGE" -c \
  'ls -d /opt/stellar-default/local/core/etc/config-settings/p*/ | sed "s|.*/p||; s|/||" | sort -n | tail -1')
docker run --rm --entrypoint cat "$IMAGE" "/opt/stellar-default/local/core/etc/config-settings/p$proto/testnet.json" |
  jq '(.updated_entry[] | select(has("state_archival")) | .state_archival) |= (.min_persistent_ttl = 100 | .min_temporary_ttl = 16)' \
    >"$WORK/short.json"
docker create --name "$CONTAINER" -p "$PORT:8000" "$IMAGE" --local --limits short --enable core,rpc >/dev/null
docker cp "$WORK/short.json" "$CONTAINER:/opt/stellar-default/local/core/etc/config-settings/p$proto/short.json"
docker start "$CONTAINER" >/dev/null
stellar network rm "$NETWORK" >/dev/null 2>&1 || true
stellar network add "$NETWORK" --rpc-url "http://localhost:$PORT/rpc" --network-passphrase "Standalone Network ; February 2017"
for _ in $(seq 60); do
  if [[ $(stellar network settings --network "$NETWORK" --output json 2>/dev/null |
    jq '[.. | objects | .min_persistent_ttl? // empty][0]') == 100 ]]; then break; fi
  sleep 5
done
[[ $(stellar network settings --network "$NETWORK" --output json 2>/dev/null |
  jq '[.. | objects | .min_persistent_ttl? // empty][0]') == 100 ]] || fail "local network did not apply the short limits"
echo "protocol $proto, image $(docker image inspect --format '{{index .RepoDigests 0}}' "$IMAGE" 2>/dev/null || echo "$IMAGE")"

step "Local keys and a short-ttl test build"
# Friendbot comes up a little after RPC reports healthy.
for role in admin reporter technician verifier; do
  for try in $(seq 30); do
    stellar keys generate "$PREFIX-$role" --network "$NETWORK" --fund --overwrite >/dev/null 2>&1 && break
    ((try < 30)) || fail "could not fund $PREFIX-$role from the local friendbot"
    sleep 5
  done
done
ADMIN=$(stellar keys address $PREFIX-admin)
REPORTER=$(stellar keys address $PREFIX-reporter)
TECH=$(stellar keys address $PREFIX-technician)
VER=$(stellar keys address $PREFIX-verifier)
cargo build -q --target wasm32v1-none --release -p water_uptime --features short-ttl
cp target/wasm32v1-none/release/water_uptime.wasm "$WORK/short-ttl.wasm"
# Leave target/ holding a normal build, not the test one.
cargo build -q --target wasm32v1-none --release -p water_uptime
CONTRACT=$(stellar contract deploy --wasm "$WORK/short-ttl.wasm" --source $PREFIX-admin --network "$NETWORK" 2>/dev/null)
echo "contract $CONTRACT"

step "One synthetic job: report, assign, rework, approve, close"
POINT=$(rand32)
call admin initialize --admin "$ADMIN" >/dev/null
call admin set_reporter --reporter "$REPORTER" --allowed true >/dev/null
call admin register_water_point --water_point_id "$POINT" --metadata_hash "$(rand32)" >/dev/null
JOB=$(call reporter report_fault --reporter "$REPORTER" --water_point_id "$POINT" --fault_reference "$(rand32)" | tr -d '"')
call admin assign_job --job_id "$JOB" --technician "$TECH" --verifier "$VER" >/dev/null
call technician submit_repair --technician "$TECH" --job_id "$JOB" --repair_reference "$(rand32)" >/dev/null
call verifier verify_repair --verifier "$VER" --job_id "$JOB" --verification_reference "$(rand32)" --approved false >/dev/null
call technician submit_repair --technician "$TECH" --job_id "$JOB" --repair_reference "$(rand32)" >/dev/null
call verifier verify_repair --verifier "$VER" --job_id "$JOB" --verification_reference "$(rand32)" --approved true >/dev/null
call admin close_job --job_id "$JOB" >/dev/null
[[ $(read_fn get_job --job_id "$JOB" | jq -r .status) == Closed ]] || fail "job $JOB did not close"
audit 0 export "$EXPORT"

step "1. Let everything archive"
wait_archived History "{\"u64\":\"$JOB\"}"
for entry in "Job {\"u64\":\"$JOB\"}" "WaterPoint {\"bytes\":\"$POINT\"}"; do
  # shellcheck disable=SC2086
  [[ $(live_until $entry) == 0 ]] || fail "$entry is not archived"
done
[[ $(stellar ledger entry fetch contract-data --contract "$CONTRACT" --instance --network "$NETWORK" --output json 2>/dev/null |
  jq '.entries[0].liveUntilLedgerSeq') == 0 ]] || fail "instance is not archived"
echo "job, history, water point and contract instance are archived"

step "2. Read and verify while archived (expect exit 2)"
[[ $(read_fn get_job_history --job_id "$JOB" | jq -c .) == "$(jq -c ".jobs[] | select(.id == $JOB) | .history" "$EXPORT")" ]] ||
  fail "archived history read through simulation differs from the export"
echo "simulated read of the archived history matches the export"
audit 2 verify "$EXPORT"

step "3. Restore, extend, verify (expect exit 0)"
audit 0 restore "$EXPORT"
audit 0 extend 300
audit 0 verify "$EXPORT"

step "4. Archive again; a submitted fault report restores the water point itself"
wait_archived WaterPoint "{\"bytes\":\"$POINT\"}"
JOB2=$(call reporter report_fault --reporter "$REPORTER" --water_point_id "$POINT" --fault_reference "$(rand32)" | tr -d '"')
[[ -n $JOB2 && $(live_until WaterPoint "{\"bytes\":\"$POINT\"}") -gt $(ledger) ]] ||
  fail "report_fault did not restore the water point"
echo "job $JOB2 reported; the water point is live again until ledger $(live_until WaterPoint "{\"bytes\":\"$POINT\"}")"

printf '\nOK: records archived, were readable and verified while archived, restored intact by\n'
printf 'audit-export.sh restore, and restored automatically by a submitted transaction.\n'
