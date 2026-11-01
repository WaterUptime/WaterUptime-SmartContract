# Audit record retention and restoration

How long WaterUptime's on-chain audit records last, what can make them unavailable, and the procedures for keeping, exporting, verifying and restoring them.

> **Prototype on Stellar testnet.** The audit trail is **not permanent or unconditionally available**. Records stay live only while someone keeps extending them. Each job's history is limited to 50 entries. Contract events are kept by RPC nodes for a short window only. The testnet itself is reset from time to time, which erases everything on it. Descriptions and evidence were never on-chain; only their fingerprints are. This document explains what that means in practice.

## What the contract stores, and for how long

Soroban charges rent for stored data. Every entry has a time to live (TTL) in ledgers (about 5 seconds each). When its TTL runs out, the entry is **archived**. An archived entry is not deleted, and simulated reads still return it today, but it must be restored before a transaction can use it.

| Entry | Holds | TTL set by the contract | Extended when |
|---|---|---|---|
| Contract instance | Admin, job counter | 30 days | Any write call |
| Contract code (Wasm) | The program | Not managed by the contract | Only by an explicit extension |
| `Job(id)` | Current state of a job | 120 days | The job is written, or read inside a **submitted** transaction |
| `History(id)` | The job's audit entries | 120 days | The same |
| `WaterPoint(id)`, `Reporter(address)` | Registry and permissions | 120 days | The same |

Two consequences matter for auditability:

- **Free reads don't extend anything.** Reads are simulated: the web app's testnet view, `stellar contract invoke --send=no`, and every `get_*` call made through RPC simulation. The TTL extension inside those calls is computed but never committed.
- **A finished job is never written again.** After a job is closed or cancelled, its `Job` and `History` entries count down from 120 days. They are archived unless someone extends them. Tested in `closing_a_job_keeps_its_records_alive_for_120_days`.

The network caps any TTL at 3,110,400 ledgers (about 180 days, testnet protocol 29, checked 2026-10-28). So no single extension keeps a record live for longer than about six months.

### What archived means

Since protocol 23, an archived persistent entry keeps its contents in the network's archive. This is what we observed with genuinely archived WaterUptime records (see [tested](#tested-on-a-local-network) below):

- **RPC reports it as archived:** `liveUntilLedgerSeq` is 0.
- **It can still be read by simulation.** That covers the web app's testnet view, `stellar contract invoke --send=no`, and `audit-export.sh export` and `verify`. They return the archived job and history unchanged, without restoring anything. This is how RPC behaves today, not a guarantee: it relies on RPC serving archived state, and later protocol changes to how archived state is stored could require restoring first.
- **A submitted transaction that touches it restores it automatically.** The submitter pays the restoration fee.
- **An explicit restore makes it live again.** Use `audit-export.sh restore`, or `stellar contract restore` directly; any account can submit it.
- **A restored entry gets only the network's minimum TTL:** about 7 days on testnet. Extend it straight after restoring.

Unit test `archived_history_is_restored_intact` covers the same behaviour in the Soroban test host.

**Not yet exercised on testnet,** because nothing there has archived. A new entry lives at least about 7 days and the contract extends its records to 120 days, so archival takes months. The first natural case will be the superseded `v0.1.0` instance (`CDRNO3…`), which nobody extends: its contract instance archives around 2026-11-26 and its job records around 2027-02-24. [#15](https://github.com/WaterUptime/WaterUptime-SmartContract/issues/15) tracks running the procedure against it. Until then, the restore command has only been run on testnet against a live entry, where it changes nothing (transaction [`38ce19eb…`](https://stellar.expert/explorer/testnet/tx/38ce19eb683b9c391d5322beb3c561083d374c732db37ecf772bb56a58e4c3b4)).

## The history length limit

Each job keeps at most `MAX_HISTORY_LEN` = 50 audit entries. The limit bounds storage cost, which grows with every rework cycle.

The contract **reserves room for the steps a job still needs to finish**. An action is accepted only if, after it, there is still space for the shortest path to `Closed` or `Cancelled`:

| Status after the action | Entries still needed to finish |
|---|---|
| `Reported`, `Assigned`, `NeedsRework`, `Verified` | 1 (cancel, or close) |
| `RepairSubmitted` | 2 (a decision, then close or cancel) |

With a limit of 50, this works out as follows:
- A job can go through **at most 23 rework cycles**.
- The 24th submission is refused with `HistoryLimitReached`, at 48 entries.
- The coordinator can still reassign once, and can always cancel.
- Approving the last accepted submission still leaves room to close.

Before this rule (contract `v0.1.1` and earlier, fixed in `v0.2.0`), a job could reach 50 entries in `NeedsRework` or `Verified`. It could then be neither cancelled nor closed, and its water point stayed blocked for good. The tests `every_job_can_still_be_finished_near_the_history_limit`, `submission_refused_when_it_would_leave_no_room_to_finish` and `history_is_bounded` cover every path at the boundary.

**When a job hits the limit:**
1. The coordinator cancels it, with a reason fingerprint.
2. A reporter opens a new job for the same water point.
3. The off-chain record of the new job should name the old job ID, because the contract does not link them.

## Transaction history and events

Every state change also emits an event (see `CONTRACT_INTERFACE.md`). RPC nodes keep events for a limited window, days rather than months, so events are not a long-term record. Anyone who needs a complete event history must run an indexer that stores events as they arrive. This project doesn't run one.

The submitted transactions themselves, including their arguments (addresses and fingerprints), are kept in the network's history archives and by explorers such as Stellar Expert, independently of contract-state TTL. That makes them a secondary source for reconstructing a job's actions. It is also why the privacy docs treat anything sent on-chain as potentially permanent. It isn't a retrieval procedure this project has tested, and a testnet reset removes it too.

## Retention procedure

`scripts/audit-export.sh` implements the routine tasks. It needs the Stellar CLI and `jq`. It works on any deployment (`CONTRACT_ID`), with any funded identity paying (`SOURCE`); that account doesn't need to be the admin.

| When | Command | What it does |
|---|---|---|
| At least every **90 days**, and after any long quiet period | `scripts/audit-export.sh extend` | Extends the instance, the code, and every job, history and water-point entry by 3,000,000 ledgers (about 173 days) |
| After jobs close or are cancelled, and at least **monthly** | `scripts/audit-export.sh export FILE` | Writes every job, its full history, the water points it references, and each entry's expiry ledger, then prints the file's SHA-256 |
| When checking a record, or after any restore | `scripts/audit-export.sh verify FILE` | Re-reads the chain and checks each exported job and history is unchanged (or, for a job that was still open, only extended). Lists the time left on every entry. Exits 1 on any difference or missing job, and 2 if everything is unchanged but some entries are archived |
| When `verify` reports archived entries | `scripts/audit-export.sh restore FILE`, then `extend` | Restores the instance, the code and every job, history and water-point entry in the export, then extends them |

Store each export and its SHA-256 off-chain, outside the browser. In a real deployment that belongs in the access-controlled evidence store, which doesn't exist yet. Reporter entries aren't enumerable on-chain and aren't covered by `extend`; re-run `set_reporter` to refresh one.

### Restoring an archived record

1. Run `verify` against the latest export. Entries shown as `ARCHIVED` (exit status 2) need restoring. `NOT LIVE` means the entry could not be found at all.
2. Run `scripts/audit-export.sh restore FILE`. It restores every entry the export covers; restoring one that is still live changes nothing.
3. Run `scripts/audit-export.sh extend` straight away. Restored entries get only the network's minimum TTL.
4. Run `verify` again. It must exit 0, and the restored jobs and histories must match the export exactly.

To restore a single entry by hand:

```bash
KEY=$(echo '{"vec":[{"symbol":"History"},{"u64":"7"}]}' | stellar xdr encode --type ScVal)
stellar contract restore --id "$CONTRACT_ID" --key-xdr "$KEY" --durability persistent \
  --source <any-funded-identity> --network testnet
```

Use `{"symbol":"Job"}` for the job record, and `{"vec":[{"symbol":"WaterPoint"},{"bytes":"<hex id>"}]}` for a water point. For the contract instance, pass `--id` with no key; for the code, pass `--wasm-hash`. Alternatively, any submitted transaction that reads the entry restores it automatically.

### Tested on a local network

**Archival, retrieval and restoration (2026-10-29), with `scripts/local-archival-test.sh`.** The script runs everything below in one go and takes about 20 minutes. Anyone with docker can repeat it.

**Setup:**
- A throwaway local network, `stellar/quickstart` with protocol 30. Its limits match testnet's, except that the minimum persistent entry lifetime is 100 ledgers, at about one ledger per second.
- A build of this contract with the `short-ttl` feature, where a "day" is 4 ledgers, so records are extended to 480 ledgers rather than 120 days. That is its only difference from a release. The default build stays byte-identical to one without the feature, and CI builds both.
- Archival, retrieval and restoration are done by the network and the Stellar CLI, not by contract code.

**Results:**
1. One synthetic job was reported, assigned, returned for rework, approved and closed (7 history entries), then exported with `audit-export.sh export`.
2. **Everything archived:** after about 480 ledgers the job, its history, its water point, the contract instance and its code all showed as archived (`liveUntilLedgerSeq` 0).
3. **Retrieval while archived:**
   - a simulated `get_job_history` returned the archived history exactly as exported;
   - `verify` reported the job unchanged and its 3 entries `ARCHIVED`, exiting with status 2.
4. **Restoration:** `restore` brought back the instance, code, job, history and water point, each for only the network's minimum of about 100 ledgers. `extend` then made them live again, and `verify` exited 0 with the history identical to the export.
5. **Automatic restoration:** after the water point archived again, a submitted `report_fault` for it succeeded and restored it without any separate step.

Limits of this test:
- **Local network:** it ran on a local network (protocol 30), not testnet (protocol 29).
- **Test build:** it used the test build rather than the released WASM.
- **Scale:** it covered one job.

### Tested on testnet

**History limit (2026-10-29, `v0.2.0` instance `CDO5G5UK…TF22Z`):** `scripts/testnet-history-limit.sh` ran job 3 through 23 rework cycles. The 24th submission was refused with `HistoryLimitReached` at 48 entries. The job was then cancelled with 49 entries, and its water point was free again.

**Export, verify and extend (2026-10-28, `v0.1.1` instance)**

All three tasks were run against the then-shared instance `CD5V5AJC…TCLX`, signing with the reporter account, which is not the admin:

- **`export`:** wrote 12 jobs and 7 water points. Every entry showed 118–119 days left.
- **`verify`:**
  - the unaltered export passed;
  - a copy with one history entry's actor changed was reported as `CHANGED`, exiting with status 1;
  - a copy with a history cut short, standing for a job that was still open at export time, was reported as extended since export.
- **`extend`:** extended 33 entries (the instance, the code and all 31 job, history and water-point entries) to ledgers 8,439,760–8,439,822. That is about 173 days, so they expire around **2027-04-19** unless extended again.

## What this strategy does not cover

- **Testnet resets.** When the testnet is reset, every deployment and its records are gone. Exports are then the only record, and they can no longer be checked against a chain. The project will redeploy and say so in the [compatibility table](https://github.com/WaterUptime/WaterUptime#compatibility).
- **Superseded deployments.** The old `v0.1.0` instance (`CDRNO3…`) isn't maintained. Its contract instance will be archived about 30 days after its last write (late November 2026), and its job records around the end of February 2027. After that, they need restoring before they can be read.
- **Off-chain records.** Descriptions, evidence and their salts are not on-chain. Today they exist only in the browser that created them. A fingerprint can prove a record unchanged only for as long as the record itself is kept.
- **Long-term custody.** Who must keep extending and exporting records in a real deployment, and for how many years, is a governance question for the [implementation pathway](https://github.com/WaterUptime/WaterUptime/blob/main/docs/implementation-pathway.md), not something the contract can guarantee.

Related: [contract #6](https://github.com/WaterUptime/WaterUptime-SmartContract/issues/6) (in-contract TTL extension entry points, so a single call can refresh one job).
