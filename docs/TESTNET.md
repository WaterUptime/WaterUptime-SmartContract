# Testnet Deployment

> **Testnet only, synthetic data only.** Do not send real funds, real water-point identifiers, or personal data to this contract. Stellar testnet is reset periodically, so this deployment may disappear and the contract ID will change when the contract is redeployed.

## Current deployment

| | |
|---|---|
| Network | Stellar testnet (`Test SDF Network ; September 2015`) |
| Contract ID | `CDRNO3OO3JCIWRNJIDRPH3UQ6ZX7GRTDQUPMJP4B5IVKUMNUNVHC3ZQO` |
| Built from | commit `f31863f` (PR #1), `soroban-sdk 27`, `stellar 27.1.0` |
| Deployed | 2026-10-27 |
| Explorer | [Stellar Lab](https://lab.stellar.org/r/testnet/contract/CDRNO3OO3JCIWRNJIDRPH3UQ6ZX7GRTDQUPMJP4B5IVKUMNUNVHC3ZQO) |

### Synthetic role accounts

| Role | Address |
|---|---|
| Admin / coordinator | `GAVHHQY6RP2VDVLJA2JQGS2ZCIUYWBEZK6P5J64F5VSHSEHXZTM6IXM5` |
| Reporter | `GBA3CLHHBT6454VSCZZQ5AQ4P6K453K3AJLHOFOEBH4VVBJOUK7MSS7H` |
| Technician | `GDPTFRPXBAXJXDUVXIAK3TEWBMW26MN4CKXFJYSOGJ4U477YMVCYCWPA` |
| Verifier | `GDNCUBP3ZHREJ4QGWUGJQH4DWGQVL6T5AKKU3YV65XCNN72AF5LIMILZ` |

Only the maintainer holds these keys. Anyone can **read** this deployment. To **write** (report, assign, submit and so on), deploy your own instance with the demo script below, where you control every role.

### Demo jobs

Both jobs are on water point `bd21852e…3ee766`.

**Job 1** went through the full lifecycle, including one rework cycle:

| Step | Transaction |
|---|---|
| `report_fault` | [d4731e1e…](https://stellar.expert/explorer/testnet/tx/d4731e1ec82d7eaa8fdcce1c045e29b4449874204836016e0cad9bf80c8770e8) |
| `assign_job` | [8a421b10…](https://stellar.expert/explorer/testnet/tx/8a421b10b0ab4d096f850bf8d1ea77ed65e536696fedd40bc50fc74f3c8728b5) |
| `submit_repair` | [05edbe5b…](https://stellar.expert/explorer/testnet/tx/05edbe5bd8484bf2951e48fc9c9d55713ba5e54e2f085f8765bb34d4b306e406) |
| `verify_repair` (rejected) | [3b744a67…](https://stellar.expert/explorer/testnet/tx/3b744a675432f151a376520af322f9e367143354b8a57938cbd1c9b3572d7527) |
| `submit_repair` (rework) | [6b6a7934…](https://stellar.expert/explorer/testnet/tx/6b6a7934a0f7dfffb5bcf781c4b2fd6cf0b23ae4822615f565a658f5b55639d2) |
| `verify_repair` (approved) | [11735f3c…](https://stellar.expert/explorer/testnet/tx/11735f3cdd6b5dc705d701496b0a24cc6dd2d00d6db076efbc6b63aed7e7fa79) |
| `close_job` | [217798f4…](https://stellar.expert/explorer/testnet/tx/217798f44f110551b63b5bc0962ec60bdd85625eddd95662e1ac1cb79a36f483) |

**Job 2** is a mistaken report, cancelled by the coordinator:

| Step | Transaction |
|---|---|
| `report_fault` | [a0b41774…](https://stellar.expert/explorer/testnet/tx/a0b41774df6c8ec0da556d12bfc70500cfb5801565212be2d54a142e047762df) |
| `cancel_job` | [790c0092…](https://stellar.expert/explorer/testnet/tx/790c0092434859b96e3aceacfbb7bccc6b2a313159100aaaded82a0616b0bcbe) |

## Reading the deployment

Read-only calls are simulated rather than submitted, so they are free. Any funded testnet key works as `--source`:

```bash
stellar keys generate reader --network testnet --fund   # once

CONTRACT=CDRNO3OO3JCIWRNJIDRPH3UQ6ZX7GRTDQUPMJP4B5IVKUMNUNVHC3ZQO
stellar contract invoke --id $CONTRACT --source reader --network testnet --send=no -- job_count
stellar contract invoke --id $CONTRACT --source reader --network testnet --send=no -- get_job --job_id 1
stellar contract invoke --id $CONTRACT --source reader --network testnet --send=no -- get_job_history --job_id 1
```

Expected results:
- `job_count` is `2`.
- Job 1: `"status":"Closed"`, with 7 history entries: `Reported, Assigned, RepairSubmitted, RepairReturned, RepairSubmitted, RepairApproved, Closed`.
- Job 2: `"status":"Cancelled"`, with 2 history entries: `Reported, Cancelled`.

## For the frontend

Generate TypeScript bindings from the deployed contract:

```bash
stellar contract bindings typescript \
  --network testnet \
  --contract-id CDRNO3OO3JCIWRNJIDRPH3UQ6ZX7GRTDQUPMJP4B5IVKUMNUNVHC3ZQO \
  --output-dir packages/water_uptime
```

The bindings include typed clients for every entry point plus the `JobStatus`, `MaintenanceJob`, `AuditEntry` and `Error` types. Error codes and event payloads are documented in [`CONTRACT_INTERFACE.md`](CONTRACT_INTERFACE.md). Map error codes to plain-language messages in the UI; never show a raw `Error(Contract, #9)` to users.

Testnet RPC: `https://soroban-testnet.stellar.org` · Network passphrase: `Test SDF Network ; September 2015`

## Running your own instance

```bash
rustup target add wasm32v1-none
scripts/testnet-demo.sh
```

The script creates four funded testnet keys (`wu-admin`, `wu-reporter`, `wu-technician`, `wu-verifier`, or set `KEY_PREFIX`). It then deploys a fresh contract with you as admin, runs one synthetic job through the full lifecycle including a rework cycle, checks that an invalid close is rejected, files and cancels a second mistaken report, and prints both jobs' final state and history.

**Expected result:** job 1 ends `Closed` with 7 history entries, and job 2 ends `Cancelled` with 2. The script exits non-zero if any step fails. Testnet RPC occasionally drops a request (`client error (SendRequest)`); if that happens, re-run the script.

## Redeploying the shared instance (maintainers)

After contract changes are merged to `main`:

1. Run `scripts/testnet-demo.sh` from `main` with the maintainer keys.
2. Update the contract ID, commit, date and transaction links in this file.
3. Tell the frontend team, since bindings must be regenerated whenever the interface changes.
