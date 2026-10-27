# Testnet Deployment

> **Testnet only, synthetic data only.** Do not send real funds, real water-point identifiers, or personal data to this contract. Stellar testnet is reset periodically, so this deployment may disappear and the contract ID will change when the contract is redeployed.

## Current deployment

| | |
|---|---|
| Network | Stellar testnet (`Test SDF Network ; September 2015`) |
| Contract ID | `CD5V5AJCWEP25CVVFQ3J5E54AFOB6JFYO4MIWBJSIPECMEM3PHZ6TCLX` |
| Source | tag [`v0.1.1`](https://github.com/WaterUptime/WaterUptime-SmartContract/tree/v0.1.1), commit `38760fa` |
| WASM | [`water_uptime_v0.1.1.wasm`](https://github.com/WaterUptime/WaterUptime-SmartContract/releases/tag/v0.1.1_water_uptime_cli28.1.0), SHA-256 `02c7734bf6457a2dd5500f1a56d034aa9192fbe6b97cc95005d73ab106d4cff8`, attested build (Stellar CLI 28.1.0, `--optimize`) |
| Deployed | 2026-10-27, transactions [b3fe6e24…](https://stellar.expert/explorer/testnet/tx/b3fe6e248c4322dcf70868b9fd3dac87303dd189200feae51cdb644f196976cf) (upload) and [4c98ea83…](https://stellar.expert/explorer/testnet/tx/4c98ea83f04b5cdb685b9453db4fdb0a69a03760d97cdb541340a93a609a94b4) (create) |
| Explorer | [Stellar Expert](https://stellar.expert/explorer/testnet/contract/CD5V5AJCWEP25CVVFQ3J5E54AFOB6JFYO4MIWBJSIPECMEM3PHZ6TCLX) (source verified) · [Stellar Lab](https://lab.stellar.org/r/testnet/contract/CD5V5AJCWEP25CVVFQ3J5E54AFOB6JFYO4MIWBJSIPECMEM3PHZ6TCLX) |

### Synthetic role accounts

| Role | Address |
|---|---|
| Admin / coordinator | `GAVHHQY6RP2VDVLJA2JQGS2ZCIUYWBEZK6P5J64F5VSHSEHXZTM6IXM5` |
| Reporter | `GBA3CLHHBT6454VSCZZQ5AQ4P6K453K3AJLHOFOEBH4VVBJOUK7MSS7H` |
| Technician | `GDPTFRPXBAXJXDUVXIAK3TEWBMW26MN4CKXFJYSOGJ4U477YMVCYCWPA` |
| Verifier | `GDNCUBP3ZHREJ4QGWUGJQH4DWGQVL6T5AKKU3YV65XCNN72AF5LIMILZ` |

Only the maintainer holds these keys. Anyone can **read** this deployment. To **write** (report, assign, submit and so on), deploy your own instance with the demo script below, where you control every role.

### Demo jobs

Both jobs are on water point `72d6e786…193d27`.

**Job 1** went through the full lifecycle, including one rework cycle:

| Step | Transaction |
|---|---|
| `initialize` | [1789ab57…](https://stellar.expert/explorer/testnet/tx/1789ab57930d26e080570f6d8f4dba494029fbb00b4796e5f91dead8bf7809e3) |
| `set_reporter` | [b81a35a0…](https://stellar.expert/explorer/testnet/tx/b81a35a05ac65154c67ff438a4098f1cf1370cc192551ff4dcaea789646691af) |
| `register_water_point` | [7dd83e94…](https://stellar.expert/explorer/testnet/tx/7dd83e944634352d12d90056111504130b06094fba702c92df2261f65e8c3106) |
| `report_fault` | [414b35cd…](https://stellar.expert/explorer/testnet/tx/414b35cd3753aa6572de37c60b5af176e0483ae73a3e668e1ac142baf82b1e4d) |
| `assign_job` | [f4637139…](https://stellar.expert/explorer/testnet/tx/f463713963080f7ec1df930c6b45cd86e79a01583ce5cdc84c7fdb076dc3e0fb) |
| `submit_repair` | [a95c8d9a…](https://stellar.expert/explorer/testnet/tx/a95c8d9aaadbf5fc2d65a4280c64f33784336bce24292fddd82650dc86afe67f) |
| `verify_repair` (rejected) | [5330f568…](https://stellar.expert/explorer/testnet/tx/5330f568f9e999c147810c59480c46a6b1f70c79f5b2fdb44474892aefb79607) |
| `submit_repair` (rework) | [d3027ade…](https://stellar.expert/explorer/testnet/tx/d3027adea895bdbbb38c3b235d96500b6ceb40ee1a8c893c0ccf6473c58fe535) |
| `verify_repair` (approved) | [c4dd8d41…](https://stellar.expert/explorer/testnet/tx/c4dd8d41a75864584a0f7725b07249edd36350bd42ee50f0e3303513fdee67bd) |
| `close_job` | [30e25c2d…](https://stellar.expert/explorer/testnet/tx/30e25c2df67038d682217ef1bff00d2996716f0c53792935d62d2e44ea8669ef) |

**Job 2** is a mistaken report, cancelled by the coordinator:

| Step | Transaction |
|---|---|
| `report_fault` | [309dd1d5…](https://stellar.expert/explorer/testnet/tx/309dd1d547708a2cf8a593d9b2191002cd7b62efa009d6d56a38d777a7454bae) |
| `cancel_job` | [68d8d9f5…](https://stellar.expert/explorer/testnet/tx/68d8d9f5724655a9e3102b2d6589a39c1740f2f19329a4532881e75da6ea746a) |

### Previous deployments

| Contract ID | Source | Status |
|---|---|---|
| `CDRNO3OO3JCIWRNJIDRPH3UQ6ZX7GRTDQUPMJP4B5IVKUMNUNVHC3ZQO` | `v0.1.0` (reproducible local build, not explorer-verified) | Superseded by the v0.1.1 deployment; left in place, read-only history |

## Reading the deployment

Read-only calls are simulated rather than submitted, so they are free. Any funded testnet key works as `--source`:

```bash
stellar keys generate reader --network testnet --fund   # once

CONTRACT=CD5V5AJCWEP25CVVFQ3J5E54AFOB6JFYO4MIWBJSIPECMEM3PHZ6TCLX
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
  --contract-id CD5V5AJCWEP25CVVFQ3J5E54AFOB6JFYO4MIWBJSIPECMEM3PHZ6TCLX \
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
