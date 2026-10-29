# Testnet Deployment

> **Testnet only, synthetic data only.** Do not send real funds, real water-point identifiers, or personal data to this contract. Stellar testnet is reset periodically, so this deployment may disappear and the contract ID will change when the contract is redeployed.

## Current deployment

| | |
|---|---|
| Network | Stellar testnet (`Test SDF Network ; September 2015`) |
| Contract ID | `CDO5G5UK4YLRCBXWJI7GSJYTPXZSZNCAQ4ILAKZHYUBA64QWWCUTF22Z` |
| Source | tag [`v0.2.0`](https://github.com/WaterUptime/WaterUptime-SmartContract/tree/v0.2.0), commit `9b14788` |
| WASM | [`water_uptime_v0.2.0.wasm`](https://github.com/WaterUptime/WaterUptime-SmartContract/releases/tag/v0.2.0_water_uptime_cli28.1.0), SHA-256 `6ac0e9d8eb4f9a317f81df5786e663b02f8d6ae835e27bd94f9bb383cdbae3dd`, attested build (Stellar CLI 28.1.0, `--optimize`) |
| Deployed | 2026-10-29, transactions [e8bbe982…](https://stellar.expert/explorer/testnet/tx/e8bbe982018d197bd58d8b6c045cde6ee240f602dc3fbaea0a681a50d3047ba2) (upload) and [8a9f4463…](https://stellar.expert/explorer/testnet/tx/8a9f4463444ed989a9e2a1da66a6f57282fe7317a2ca7149138b53be4aaf0c7c) (create) |
| Explorer | [Stellar Expert](https://stellar.expert/explorer/testnet/contract/CDO5G5UK4YLRCBXWJI7GSJYTPXZSZNCAQ4ILAKZHYUBA64QWWCUTF22Z) (source verified) · [Stellar Lab](https://lab.stellar.org/r/testnet/contract/CDO5G5UK4YLRCBXWJI7GSJYTPXZSZNCAQ4ILAKZHYUBA64QWWCUTF22Z) |

### Synthetic role accounts

| Role | Address |
|---|---|
| Admin / coordinator | `GAVHHQY6RP2VDVLJA2JQGS2ZCIUYWBEZK6P5J64F5VSHSEHXZTM6IXM5` |
| Reporter | `GBA3CLHHBT6454VSCZZQ5AQ4P6K453K3AJLHOFOEBH4VVBJOUK7MSS7H` |
| Technician | `GDPTFRPXBAXJXDUVXIAK3TEWBMW26MN4CKXFJYSOGJ4U477YMVCYCWPA` |
| Verifier | `GDNCUBP3ZHREJ4QGWUGJQH4DWGQVL6T5AKKU3YV65XCNN72AF5LIMILZ` |

Only the maintainer holds these keys. Anyone can **read** this deployment. To **write** (report, assign, submit and so on), deploy your own instance with the demo script below, where you control every role.

### Demo jobs

All three jobs are on water point `99d18cce…132b67`.

**Job 1** went through the full lifecycle, including one rework cycle:

| Step | Transaction |
|---|---|
| `initialize` | [820382c9…](https://stellar.expert/explorer/testnet/tx/820382c95aa2cb529febd274a836c40baaf783d7e748b9ebfa69c6e355c25988) |
| `set_reporter` | [e3f00240…](https://stellar.expert/explorer/testnet/tx/e3f00240e919588c037d3179ac14bf217a1b365f8c932e7b3cb3ae5d45a8f009) |
| `register_water_point` | [6723ab82…](https://stellar.expert/explorer/testnet/tx/6723ab822ef578637361fa001a50b495ad40a9201ad360a78c8e6f6f21c39b30) |
| `report_fault` | [f0591061…](https://stellar.expert/explorer/testnet/tx/f059106178e6cac3c16fae1552186574d8dd99edc27b9e1b9460e58e4322124a) |
| `assign_job` | [7b92867c…](https://stellar.expert/explorer/testnet/tx/7b92867c291afa294abad447007316af6c91a040ed4cdbb82be21a0b856a93d4) |
| `submit_repair` | [e6471298…](https://stellar.expert/explorer/testnet/tx/e6471298affd2d6ac5d2529d701d7b81dbbf8f1c72907f81a01b407fe6c01c97) |
| `verify_repair` (rejected) | [b126451b…](https://stellar.expert/explorer/testnet/tx/b126451ba0a976bc03543082601e6d3154463359fa642ec1b747a0fe2a56f573) |
| `submit_repair` (rework) | [35531243…](https://stellar.expert/explorer/testnet/tx/3553124322eeb0efb287e9a871b634c0c5b358918e26099afd3e7fb26921bba3) |
| `verify_repair` (approved) | [ee50b811…](https://stellar.expert/explorer/testnet/tx/ee50b811123090ea78c72709ad017682cad17a4f51a565c6faa98d4d377fcf19) |
| `close_job` | [0ac61b05…](https://stellar.expert/explorer/testnet/tx/0ac61b05592bae9551d626c61709b523b2c02667f085799ec14c8b41d321f7a8) |

**Job 2** is a mistaken report, cancelled by the coordinator:

| Step | Transaction |
|---|---|
| `report_fault` | [f1f7892e…](https://stellar.expert/explorer/testnet/tx/f1f7892e54d671d75126cc407277b05526506d5a09e14151de5445f9fffee389) |
| `cancel_job` | [1aa64e1f…](https://stellar.expert/explorer/testnet/tx/1aa64e1f91f9a4693b9babd6e1a5c3f71a8a09edf548b72406550a464388dbcb) |

**Job 3** checks the history limit on-chain (`scripts/testnet-history-limit.sh`, see [RETENTION.md](RETENTION.md#the-history-length-limit)):
- 23 rework cycles were accepted;
- the 24th submission was refused with `HistoryLimitReached` (#13) at 48 entries;
- the coordinator then cancelled the job, with 49 entries, and the water point was free again.

### Previous deployments

| Contract ID | Source | Status |
|---|---|---|
| `CD5V5AJCWEP25CVVFQ3J5E54AFOB6JFYO4MIWBJSIPECMEM3PHZ6TCLX` | `v0.1.1` (attested, source verified), WASM `02c7734b…` | Superseded by v0.2.0 on 2026-10-29. Has the [history-limit bug](RETENTION.md#the-history-length-limit). Holds 12 jobs, all closed or cancelled; their records were extended to about 2027-04-19 and will not be extended again |
| `CDRNO3OO3JCIWRNJIDRPH3UQ6ZX7GRTDQUPMJP4B5IVKUMNUNVHC3ZQO` | `v0.1.0` (reproducible local build, not explorer-verified) | Superseded by the v0.1.1 deployment; not maintained, so its records will be archived (see [RETENTION.md](RETENTION.md)) |

## Reading the deployment

Read-only calls are simulated rather than submitted, so they are free. Any funded testnet key works as `--source`:

```bash
stellar keys generate reader --network testnet --fund   # once

CONTRACT=CDO5G5UK4YLRCBXWJI7GSJYTPXZSZNCAQ4ILAKZHYUBA64QWWCUTF22Z
stellar contract invoke --id $CONTRACT --source reader --network testnet --send=no -- job_count
stellar contract invoke --id $CONTRACT --source reader --network testnet --send=no -- get_job --job_id 1
stellar contract invoke --id $CONTRACT --source reader --network testnet --send=no -- get_job_history --job_id 1
```

Expected results:
- `job_count` is at least `3`. Later tests, such as the frontend's wallet walkthrough, add more jobs.
- Job 1: `"status":"Closed"`, with 7 history entries: `Reported, Assigned, RepairSubmitted, RepairReturned, RepairSubmitted, RepairApproved, Closed`.
- Job 2: `"status":"Cancelled"`, with 2 history entries: `Reported, Cancelled`.
- Job 3: `"status":"Cancelled"`, with 49 history entries: the history-limit check.

## For the frontend

Generate TypeScript bindings from the deployed contract:

```bash
stellar contract bindings typescript \
  --network testnet \
  --contract-id CDO5G5UK4YLRCBXWJI7GSJYTPXZSZNCAQ4ILAKZHYUBA64QWWCUTF22Z \
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

1. Bump the crate version in `contracts/water_uptime/Cargo.toml` (in the PR), then push a matching tag, e.g. `git tag -a v0.1.2 && git push origin v0.1.2`. The **Build and Release** workflow builds and attests the WASM and publishes a release.
2. Download the release asset, check its hash and attestation, and deploy **that file**:
   `stellar contract deploy --wasm water_uptime_vX.Y.Z.wasm --source wu-admin --network testnet`
3. Run the demo against it: `CONTRACT_ID=<new id> scripts/testnet-demo.sh`.
4. Check that Stellar Expert shows the source as verified.
5. Update this file and the README, move the old ID to *Previous deployments*, and update the frontend's default contract ID and the hub's [compatibility table](https://github.com/WaterUptime/WaterUptime#compatibility).
