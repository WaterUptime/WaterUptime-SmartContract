#!/usr/bin/env bash
# Run a synthetic WaterUptime maintenance job end to end on Stellar TESTNET.
#
# Deploys a fresh contract instance (you become its admin), then walks one job
# through report -> assign -> submit -> reject -> resubmit -> approve -> close,
# and prints the final job state and audit history.
#
# Synthetic data only: every ID and evidence reference is random bytes.
# Never point this at mainnet or use real water-point or personal data.
#
# Usage (from the repository root):
#   scripts/testnet-demo.sh
#
# Environment:
#   KEY_PREFIX   prefix for the four Stellar CLI keys (default: wu)
#   CONTRACT_ID  reuse an existing, not-yet-initialised deployment instead of deploying

set -euo pipefail

NETWORK=testnet
PREFIX=${KEY_PREFIX:-wu}
ADMIN=$PREFIX-admin
REPORTER=$PREFIX-reporter
TECHNICIAN=$PREFIX-technician
VERIFIER=$PREFIX-verifier
WASM=target/wasm32v1-none/release/water_uptime.wasm

rand32() { openssl rand -hex 32; }

step() { printf '\n==> %s\n' "$*"; }

invoke() {
  local source=$1
  shift
  stellar contract invoke --id "$CONTRACT" --source "$source" --network "$NETWORK" -- "$@"
}

read_only() {
  stellar contract invoke --id "$CONTRACT" --source "$ADMIN" --network "$NETWORK" --send=no -- "$@"
}

step "Ensuring funded testnet keys ($PREFIX-*)"
for key in "$ADMIN" "$REPORTER" "$TECHNICIAN" "$VERIFIER"; do
  if ! stellar keys address "$key" >/dev/null 2>&1; then
    stellar keys generate "$key" --network "$NETWORK" --fund
  fi
  echo "$key: $(stellar keys address "$key")"
done

if [[ -n "${CONTRACT_ID:-}" ]]; then
  CONTRACT=$CONTRACT_ID
else
  step "Building and deploying a fresh contract instance"
  stellar contract build
  CONTRACT=$(stellar contract deploy --wasm "$WASM" --source "$ADMIN" --network "$NETWORK")
fi
echo "contract: $CONTRACT"

WATER_POINT=$(rand32)

step "Initialise and set up"
invoke "$ADMIN" initialize --admin "$ADMIN"
invoke "$ADMIN" set_reporter --reporter "$REPORTER" --allowed true
invoke "$ADMIN" register_water_point --water_point_id "$WATER_POINT" --metadata_hash "$(rand32)"

step "Report fault"
JOB=$(invoke "$REPORTER" report_fault --reporter "$REPORTER" \
  --water_point_id "$WATER_POINT" --fault_reference "$(rand32)")
echo "job_id: $JOB"

step "Assign technician and verifier"
invoke "$ADMIN" assign_job --job_id "$JOB" --technician "$TECHNICIAN" --verifier "$VERIFIER"

step "Submit repair, verifier returns it for rework"
invoke "$TECHNICIAN" submit_repair --technician "$TECHNICIAN" --job_id "$JOB" --repair_reference "$(rand32)"
invoke "$VERIFIER" verify_repair --verifier "$VERIFIER" --job_id "$JOB" \
  --verification_reference "$(rand32)" --approved false

step "Resubmit repair, verifier approves"
invoke "$TECHNICIAN" submit_repair --technician "$TECHNICIAN" --job_id "$JOB" --repair_reference "$(rand32)"
invoke "$VERIFIER" verify_repair --verifier "$VERIFIER" --job_id "$JOB" \
  --verification_reference "$(rand32)" --approved true

step "Close job"
invoke "$ADMIN" close_job --job_id "$JOB"

step "Negative check: technician cannot close the job again (expect Error #9)"
if invoke "$TECHNICIAN" close_job --job_id "$JOB" >/dev/null 2>&1; then
  echo "UNEXPECTED: close_job succeeded" >&2
  exit 1
fi
echo "rejected as expected"

step "Final job state"
read_only get_job --job_id "$JOB"

step "Audit history"
read_only get_job_history --job_id "$JOB"

printf '\nDone. Contract %s, job %s, expected status "Closed" with 7 history entries.\n' "$CONTRACT" "$JOB"
