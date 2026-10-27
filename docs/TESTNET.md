# Testnet Deployment

> **Testnet only, synthetic data only.** Do not send real funds, real water-point identifiers, or personal data to this contract. Stellar testnet is reset periodically, so this deployment may disappear and the contract ID will change when the contract is redeployed.

## Current deployment

| | |
|---|---|
| Network | Stellar testnet (`Test SDF Network ; September 2015`) |
| Contract ID | `CAB2RVLLJCMV5FWVYGEUB7GMCZHCAQBOSBL7LQPX6V4UQ3TFA5GHH646` |
| Built from | commit `9d1eb49` (PR #1), `soroban-sdk 27`, `stellar 27.1.0` |
| Deployed | 2026-10-27 |
| Explorer | [Stellar Lab](https://lab.stellar.org/r/testnet/contract/CAB2RVLLJCMV5FWVYGEUB7GMCZHCAQBOSBL7LQPX6V4UQ3TFA5GHH646) |

### Synthetic role accounts

| Role | Address |
|---|---|
| Admin / coordinator | `GAVHHQY6RP2VDVLJA2JQGS2ZCIUYWBEZK6P5J64F5VSHSEHXZTM6IXM5` |
| Reporter | `GBA3CLHHBT6454VSCZZQ5AQ4P6K453K3AJLHOFOEBH4VVBJOUK7MSS7H` |
| Technician | `GDPTFRPXBAXJXDUVXIAK3TEWBMW26MN4CKXFJYSOGJ4U477YMVCYCWPA` |
| Verifier | `GDNCUBP3ZHREJ4QGWUGJQH4DWGQVL6T5AKKU3YV65XCNN72AF5LIMILZ` |

Only the maintainer holds these keys. Anyone can **read** this deployment. To **write** (report, assign, submit and so on), deploy your own instance with the demo script below, where you control every role.

### Demo job

Job `1` on water point `34a1e96a…698042` went through the full lifecycle, including one rework cycle:

| Step | Transaction |
|---|---|
| `report_fault` | [6af6df4a…](https://stellar.expert/explorer/testnet/tx/6af6df4a90c368f568122fd9f7a9194102e781fe24d39af32c009dee6e243835) |
| `assign_job` | [159ff157…](https://stellar.expert/explorer/testnet/tx/159ff15783bf45886691058be466f4745c0c6d7baa2859aa651e6c5bb7cbd567) |
| `submit_repair` | [1999a1ef…](https://stellar.expert/explorer/testnet/tx/1999a1ef93fcd26e7c8dd338b65712dc5279a22092674077ab60cd8da06df01d) |
| `verify_repair` (rejected) | [7f2e9541…](https://stellar.expert/explorer/testnet/tx/7f2e9541365a5bbe97a103fb95b8a15d8f1d9a5d25f54407c7262c56cd429cc4) |
| `submit_repair` (rework) | [04729fe9…](https://stellar.expert/explorer/testnet/tx/04729fe9a47f17c5b417d4fedbbebf7b3a3a7323765e204c007820bfdaeea590) |
| `verify_repair` (approved) | [d00afe05…](https://stellar.expert/explorer/testnet/tx/d00afe05f07e6f9bbb8264a3a417fa4d46ffae3f3bb7592fb0d597b4482e5a66) |
| `close_job` | [a107bb39…](https://stellar.expert/explorer/testnet/tx/a107bb3990e8a78e0566d54b14f00ce9fb830ec1706bc905af94c1f18724d30d) |

## Reading the deployment

Read-only calls are simulated rather than submitted, so they are free. Any funded testnet key works as `--source`:

```bash
stellar keys generate reader --network testnet --fund   # once

CONTRACT=CAB2RVLLJCMV5FWVYGEUB7GMCZHCAQBOSBL7LQPX6V4UQ3TFA5GHH646
stellar contract invoke --id $CONTRACT --source reader --network testnet --send=no -- job_count
stellar contract invoke --id $CONTRACT --source reader --network testnet --send=no -- get_job --job_id 1
stellar contract invoke --id $CONTRACT --source reader --network testnet --send=no -- get_job_history --job_id 1
```

`get_job` for job 1 should return `"status":"Closed"`, and `get_job_history` should return 7 entries: `Reported, Assigned, RepairSubmitted, RepairReturned, RepairSubmitted, RepairApproved, Closed`.

## For the frontend

Generate TypeScript bindings from the deployed contract:

```bash
stellar contract bindings typescript \
  --network testnet \
  --contract-id CAB2RVLLJCMV5FWVYGEUB7GMCZHCAQBOSBL7LQPX6V4UQ3TFA5GHH646 \
  --output-dir packages/water_uptime
```

The bindings include typed clients for every entry point plus the `JobStatus`, `MaintenanceJob`, `AuditEntry` and `Error` types. Error codes and event payloads are documented in [`CONTRACT_INTERFACE.md`](CONTRACT_INTERFACE.md). Map error codes to plain-language messages in the UI; never show a raw `Error(Contract, #9)` to users.

Testnet RPC: `https://soroban-testnet.stellar.org` · Network passphrase: `Test SDF Network ; September 2015`

## Running your own instance

```bash
rustup target add wasm32v1-none
scripts/testnet-demo.sh
```

The script creates four funded testnet keys (`wu-admin`, `wu-reporter`, `wu-technician`, `wu-verifier`, or set `KEY_PREFIX`). It then deploys a fresh contract with you as admin, runs one synthetic job through the full lifecycle including a rework cycle, checks that an invalid close is rejected, and prints the final state and history.

**Expected result:** the script ends with `expected status "Closed" with 7 history entries`. It exits non-zero if any step fails.

## Redeploying the shared instance (maintainers)

After contract changes are merged to `main`:

1. Run `scripts/testnet-demo.sh` from `main` with the maintainer keys.
2. Update the contract ID, commit, date and transaction links in this file.
3. Tell the frontend team, since bindings must be regenerated whenever the interface changes.
