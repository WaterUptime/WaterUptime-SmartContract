#!/usr/bin/env bash
# Export, verify and keep alive the audit records of a WaterUptime deployment
# on Stellar TESTNET. See docs/RETENTION.md for when and why to use each.
#
#   scripts/audit-export.sh export [FILE]   write every job, its history, the
#                                           water points they reference, and
#                                           each entry's expiry ledger to FILE
#   scripts/audit-export.sh verify FILE     re-read the chain and check that
#                                           every job and history in FILE is
#                                           unchanged; report time left on each.
#                                           Exits 1 if anything changed or is
#                                           missing, 2 if unchanged but archived
#   scripts/audit-export.sh extend [N]      extend the instance, code and every
#                                           job, history and water-point entry
#                                           by N ledgers (default 3000000, ~173
#                                           days; the network maximum is ~180)
#   scripts/audit-export.sh restore FILE    restore the instance, code and every
#                                           job, history and water-point entry
#                                           in FILE that has been archived
#
# Reads are simulated and free. `extend` and `restore` submit transactions; any
# funded account may pay for them, it does not need to be the admin.
#
# Environment:
#   CONTRACT_ID  deployment to read (default: the shared testnet instance)
#   SOURCE       Stellar CLI identity used to simulate and pay (default: wu-reporter)
#   NETWORK      Stellar CLI network (default: testnet; scripts/local-archival-test.sh
#                uses a local one)
#
# Requires: stellar CLI, jq, sha256sum. Testnet or a local network only.

set -euo pipefail

NETWORK=${NETWORK:-testnet}
CONTRACT=${CONTRACT_ID:-CDO5G5UK4YLRCBXWJI7GSJYTPXZSZNCAQ4ILAKZHYUBA64QWWCUTF22Z}
SOURCE=${SOURCE:-wu-reporter}
DAY=17280

read_fn() {
  stellar contract invoke --id "$CONTRACT" --source "$SOURCE" --network "$NETWORK" --send=no -- "$@" 2>/dev/null
}

# Base64 XDR of a DataKey, e.g. key_xdr Job '{"u64":"3"}'.
key_xdr() {
  jq -cn --arg v "$1" --argjson a "$2" '{vec: [{symbol: $v}, $a]}' | stellar xdr encode --type ScVal
}

# Expiry ledger of one persistent entry ("" if it cannot be read live).
live_until() {
  stellar ledger entry fetch contract-data --contract "$CONTRACT" --key-xdr "$(key_xdr "$1" "$2")" \
    --durability persistent --network "$NETWORK" --output json 2>/dev/null |
    jq -r '.entries[0].liveUntilLedgerSeq // empty'
}

latest_ledger() {
  stellar ledger latest --network "$NETWORK" 2>/dev/null | awk '/^Sequence:/ {print $2}'
}

# Current state of every job as one JSON document on stdout.
snapshot() {
  local count ledger id point jobs="[]" points="[]" seen=" "
  ledger=$(latest_ledger)
  count=$(read_fn job_count)
  for ((id = 1; id <= count; id++)); do
    local job history
    job=$(read_fn get_job --job_id "$id")
    history=$(read_fn get_job_history --job_id "$id")
    jobs=$(jq -c --argjson job "$job" --argjson history "$history" \
      --arg jt "$(live_until Job "{\"u64\":\"$id\"}")" \
      --arg ht "$(live_until History "{\"u64\":\"$id\"}")" \
      '. + [{id: $job.id, job: $job, history: $history,
             live_until: {job: ($jt | tonumber? // null), history: ($ht | tonumber? // null)}}]' <<<"$jobs")
    point=$(jq -r .water_point_id <<<"$job")
    if [[ $seen != *" $point "* ]]; then
      seen+="$point "
      points=$(jq -c --argjson p "$(read_fn get_water_point --water_point_id "$point")" \
        --arg t "$(live_until WaterPoint "{\"bytes\":\"$point\"}")" \
        '. + [{point: $p, live_until: ($t | tonumber? // null)}]' <<<"$points")
    fi
  done
  jq -n --arg contract "$CONTRACT" --arg network "$NETWORK" --argjson ledger "$ledger" \
    --arg at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" --argjson count "$count" \
    --argjson jobs "$jobs" --argjson points "$points" \
    '{contract: $contract, network: $network, exported_at: $at, ledger: $ledger,
      job_count: $count, jobs: $jobs, water_points: $points}'
}

# One line per entry: time left before it is archived.
report_ttl() {
  jq -r --argjson day "$DAY" '
    .ledger as $now
    | def left($t): if $t == null then "NOT LIVE (archived or missing)"
                    elif $t == 0 then "ARCHIVED; run restore"
                    elif $t < $now then "EXPIRED after ledger \($t), archived at the next eviction scan; run restore"
                    else "\((($t - $now) / $day) | floor) days left (\($t - $now) ledgers, until ledger \($t))" end;
    (.jobs[] | "job \(.id) [\(.job.status)]: record \(left(.live_until.job)); history (\(.history | length) entries) \(left(.live_until.history))"),
    (.water_points[] | "water point \(.point.id[0:12])…: \(left(.live_until))")' "$1"
}

cmd=${1:-}
case $cmd in
export)
  out=${2:-audit-$CONTRACT-$(date -u +%Y%m%dT%H%M%SZ).json}
  snapshot >"$out"
  echo "Exported $(jq '.jobs | length' "$out") jobs from $CONTRACT at ledger $(jq .ledger "$out") to $out"
  report_ttl "$out"
  echo "sha256 $(sha256sum "$out" | cut -d' ' -f1)  (store this with the file)"
  ;;
verify)
  file=${2:?usage: audit-export.sh verify FILE}
  CONTRACT=$(jq -r .contract "$file")
  now=$(mktemp)
  trap 'rm -f "$now"' EXIT
  snapshot >"$now"
  failed=0
  while read -r id; do
    old=$(jq -c --argjson id "$id" '.jobs[] | select(.id == $id) | {job, history}' "$file")
    new=$(jq -c --argjson id "$id" '.jobs[] | select(.id == $id) | {job, history}' "$now")
    if [[ -z $new ]]; then
      echo "job $id: MISSING on chain"
      failed=1
    elif [[ $old == "$new" ]]; then
      echo "job $id: unchanged ($(jq '.history | length' <<<"$old") history entries)"
    else
      # A job that was still open when exported may legitimately have moved on;
      # its exported history must still be a prefix of the current one.
      if jq -e --argjson o "$old" --argjson n "$new" \
        '$o.history == $n.history[0:($o.history | length)]' >/dev/null <<<"null"; then
        echo "job $id: history extended since export (earlier entries unchanged)"
      else
        echo "job $id: CHANGED (exported history is not a prefix of the chain's)"
        failed=1
      fi
    fi
  done < <(jq '.jobs[].id' "$file")
  echo "Expiry now:"
  report_ttl "$now"
  archived=$(report_ttl "$now" | { grep -o -e ARCHIVED -e EXPIRED || true; } | wc -l)
  if ((failed == 0 && archived > 0)); then
    echo "Content unchanged, but $archived entries are archived: run restore $file, then extend."
    exit 2
  fi
  exit $failed
  ;;
extend)
  ledgers=${2:-3000000}
  extend_ttl() { stellar contract extend --source "$SOURCE" --network "$NETWORK" \
    --ledgers-to-extend "$ledgers" --durability persistent --ttl-ledger-only "$@" 2>/dev/null; }
  ext() { extend_ttl --id "$CONTRACT" "$@"; }
  echo "instance: until ledger $(ext)"
  # The code entry is shared by every instance of the same Wasm.
  wasm=$(stellar contract info hash --contract-id "$CONTRACT" --network "$NETWORK" 2>/dev/null)
  echo "code ${wasm:0:12}…: until ledger $(extend_ttl --wasm-hash "$wasm")"
  count=$(read_fn job_count)
  declare -A done_points=()
  for ((id = 1; id <= count; id++)); do
    echo "job $id: until ledger $(ext --key-xdr "$(key_xdr Job "{\"u64\":\"$id\"}")")"
    echo "history $id: until ledger $(ext --key-xdr "$(key_xdr History "{\"u64\":\"$id\"}")")"
    point=$(read_fn get_job --job_id "$id" | jq -r .water_point_id)
    if [[ -z ${done_points[$point]:-} ]]; then
      done_points[$point]=1
      echo "water point ${point:0:12}…: until ledger $(ext --key-xdr "$(key_xdr WaterPoint "{\"bytes\":\"$point\"}")")"
    fi
  done
  ;;
restore)
  file=${2:?usage: audit-export.sh restore FILE}
  CONTRACT=$(jq -r .contract "$file")
  # A restored entry gets only the network's minimum TTL (~7 days on testnet),
  # so run `extend` straight afterwards. Restoring a live entry changes nothing.
  rst() {
    stellar contract restore --source "$SOURCE" --network "$NETWORK" --durability persistent "$@" 2>&1 |
      awk '/New ttl ledger/ {print $NF; found = 1} END {if (!found) print "not restored"}'
  }
  echo "instance: live until ledger $(rst --id "$CONTRACT")"
  wasm=$(stellar contract info hash --contract-id "$CONTRACT" --network "$NETWORK" 2>/dev/null)
  echo "code ${wasm:0:12}…: live until ledger $(rst --wasm-hash "$wasm")"
  while read -r id; do
    echo "job $id: live until ledger $(rst --id "$CONTRACT" --key-xdr "$(key_xdr Job "{\"u64\":\"$id\"}")")"
    echo "history $id: live until ledger $(rst --id "$CONTRACT" --key-xdr "$(key_xdr History "{\"u64\":\"$id\"}")")"
  done < <(jq '.jobs[].id' "$file")
  while read -r point; do
    echo "water point ${point:0:12}…: live until ledger $(rst --id "$CONTRACT" --key-xdr "$(key_xdr WaterPoint "{\"bytes\":\"$point\"}")")"
  done < <(jq -r '.water_points[].point.id' "$file")
  echo "Now run: $0 extend, then $0 verify $file"
  ;;
*)
  sed -n '2,30p' "$0" | sed 's/^# \{0,1\}//'
  exit 2
  ;;
esac
