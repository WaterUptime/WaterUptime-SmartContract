#!/usr/bin/env bash
# Check the history-limit rule on Stellar TESTNET (see docs/RETENTION.md).
#
# Takes one synthetic job through the maximum number of rework cycles the
# contract accepts, checks that the next submission is refused with
# HistoryLimitReached (#13) while the job can still be cancelled, and that the
# water point is free afterwards. About 50 transactions; takes a few minutes.
#
# Usage (from the repository root):
#   CONTRACT_ID=C... WATER_POINT=<hex id> scripts/testnet-history-limit.sh
#
# Needs the same four identities as testnet-demo.sh (KEY_PREFIX, default wu),
# with <prefix>-admin as the contract admin and <prefix>-reporter authorised.
# WATER_POINT must be registered, active and without an open job.
# Synthetic data only. Testnet only.

set -euo pipefail

NETWORK=testnet
PREFIX=${KEY_PREFIX:-wu}
CONTRACT=${CONTRACT_ID:?set CONTRACT_ID}
POINT=${WATER_POINT:?set WATER_POINT}
TECH_ADDR=$(stellar keys address "$PREFIX-technician")
VER_ADDR=$(stellar keys address "$PREFIX-verifier")
REP_ADDR=$(stellar keys address "$PREFIX-reporter")

rand32() { openssl rand -hex 32; }
call() {
  local source=$1
  shift
  stellar contract invoke --id "$CONTRACT" --source "$PREFIX-$source" --network "$NETWORK" -- "$@" 2>/dev/null
}
read_fn() { stellar contract invoke --id "$CONTRACT" --source "$PREFIX-reporter" --network "$NETWORK" --send=no -- "$@" 2>/dev/null; }
history_len() { read_fn get_job_history --job_id "$1" | jq length; }
fail() {
  echo "FAIL: $*" >&2
  exit 1
}

JOB=$(call reporter report_fault --reporter "$REP_ADDR" --water_point_id "$POINT" --fault_reference "$(rand32)" | tr -d '"')
call admin assign_job --job_id "$JOB" --technician "$TECH_ADDR" --verifier "$VER_ADDR" >/dev/null
echo "job $JOB reported and assigned"

cycles=0
while call technician submit_repair --technician "$TECH_ADDR" --job_id "$JOB" --repair_reference "$(rand32)" >/dev/null; do
  call verifier verify_repair --verifier "$VER_ADDR" --job_id "$JOB" --verification_reference "$(rand32)" --approved false >/dev/null ||
    fail "return for rework refused after $cycles cycles"
  cycles=$((cycles + 1))
  echo "  cycle $cycles: $(history_len "$JOB") entries"
  ((cycles <= 50)) || fail "no limit after 50 cycles"
done

len=$(history_len "$JOB")
echo "submission $((cycles + 1)) refused at $len entries"
# Confirm the refusal is the history limit, not something else.
err=$(stellar contract invoke --id "$CONTRACT" --source "$PREFIX-technician" --network "$NETWORK" --send=no -- \
  submit_repair --technician "$TECH_ADDR" --job_id "$JOB" --repair_reference "$(rand32)" 2>&1 || true)
grep -q "Error(Contract, #13)" <<<"$err" || fail "expected HistoryLimitReached, got: ${err%%$'\n'*}"

call admin cancel_job --job_id "$JOB" --reason_reference "$(rand32)" >/dev/null || fail "cancel refused at the limit"
status=$(read_fn get_job --job_id "$JOB" | jq -r .status)
final=$(history_len "$JOB")
open=$(read_fn get_water_point --water_point_id "$POINT" | jq -r .open_job)
[[ $status == Cancelled && $final -le 50 && $open == null ]] || fail "status $status, $final entries, open_job $open"
echo "OK: $cycles rework cycles accepted, submission refused with HistoryLimitReached at $len entries,"
echo "    job $JOB cancelled with $final entries, water point free again"
