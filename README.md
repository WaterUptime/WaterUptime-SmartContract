# WaterUptime — Smart Contract

Soroban (Stellar) contract for **WaterUptime**: open-source maintenance and verification infrastructure for community water systems.

The contract enforces an auditable repair workflow for a water point — fault report, assignment, repair evidence, independent verification, and close-out — and keeps a readable on-chain history of who did what and when.

> **Status:** experimental prototype. Testnet and synthetic data only. Not audited. No real funds, no mainnet, no personal data. No government, NGO, or UNDP partnership is claimed.

Related repositories: [WaterUptime](https://github.com/WaterUptime/WaterUptime) (project docs) · [WaterUptime-Frontend](https://github.com/WaterUptime/WaterUptime-Frontend) (web app).

## Lifecycle

```text
Reported ─▶ Assigned ─▶ RepairSubmitted ─▶ Verified ─▶ Closed
                ▲              │
                │              ▼
                └────────  NeedsRework   (reject → rework → resubmit)

Reported / Assigned / NeedsRework ──cancel──▶ Cancelled   (mistaken or duplicate report)
```

| Action | Caller (must sign) | Allowed from | Result |
|---|---|---|---|
| `report_fault` | authorised reporter | water point registered, active, no open job | `Reported` |
| `assign_job` | admin / coordinator | `Reported`, `Assigned`, `NeedsRework` | `Assigned` |
| `submit_repair` | assigned technician | `Assigned`, `NeedsRework` | `RepairSubmitted` |
| `verify_repair(approved=true)` | assigned verifier | `RepairSubmitted` | `Verified` |
| `verify_repair(approved=false)` | assigned verifier | `RepairSubmitted` | `NeedsRework` |
| `close_job` | admin / coordinator | `Verified` | `Closed` |
| `cancel_job` | admin / coordinator | `Reported`, `Assigned`, `NeedsRework` | `Cancelled` |

### Invariants enforced

- Only the admin (coordinator) can register water points, manage reporters, assign jobs, and close or cancel jobs.
- Reports must reference a registered, **active** water point, and only one job may be open per water point at a time.
- The technician and verifier must be different addresses.
- Only the assigned technician can submit evidence. Only the assigned verifier can approve or reject it.
- A job cannot close until it is verified. `Closed` and `Cancelled` are terminal: those jobs cannot be reassigned or modified.
- A job cannot be cancelled while repair evidence awaits verification; the verifier decides first. Cancelling records a reason fingerprint and frees the water point for new reports.
- Rework never erases history: every transition appends an `AuditEntry` (action, actor, evidence reference, timestamp). History is capped at `MAX_HISTORY_LEN` (50) entries per job.
- Every state change emits an event. See [`docs/CONTRACT_INTERFACE.md`](docs/CONTRACT_INTERFACE.md).
- Revoking a reporter does not affect jobs they already created.

### What the contract does *not* do

It records **authorised claims**; it cannot prove a repair physically happened or that water is safe. Names, phone numbers, GPS coordinates, photos, and documents stay off-chain. Only opaque 32-byte IDs and evidence fingerprints (hashes) are stored. A hash of low-entropy data can be guessed, so salt off-chain records before hashing.

## Layout

```text
contracts/water_uptime/src/
├── lib.rs       # contract entry points and state machine
├── types.rs     # WaterPoint, MaintenanceJob, JobStatus, AuditEntry, Error, storage keys
├── events.rs    # #[contractevent] definitions
├── storage.rs   # storage access and TTL management
└── test.rs      # unit tests (positive, negative, auth, events)
scripts/testnet-demo.sh   # deploy + end-to-end synthetic job on testnet
docs/                     # interface reference and testnet deployment
```

## Development

Requirements: Rust (stable) with the `wasm32v1-none` target, and the [Stellar CLI](https://developers.stellar.org/docs/tools/cli) (built and tested with `stellar 27.1.0` and `soroban-sdk 27`).

```bash
rustup target add wasm32v1-none
cargo test                  # run unit tests
stellar contract build      # build target/wasm32v1-none/release/water_uptime.wasm
cargo fmt --all && cargo clippy --all-targets -- -D warnings
```

CI runs fmt, clippy, tests, and a WASM build on every push and PR.

## Testnet (synthetic data only)

A shared instance is deployed on Stellar testnet, and its source is **verified on Stellar Expert**.

| | |
|---|---|
| Contract address | [`CD5V5AJCWEP25CVVFQ3J5E54AFOB6JFYO4MIWBJSIPECMEM3PHZ6TCLX`](https://stellar.expert/explorer/testnet/contract/CD5V5AJCWEP25CVVFQ3J5E54AFOB6JFYO4MIWBJSIPECMEM3PHZ6TCLX) |
| Explorer | [Stellar Expert](https://stellar.expert/explorer/testnet/contract/CD5V5AJCWEP25CVVFQ3J5E54AFOB6JFYO4MIWBJSIPECMEM3PHZ6TCLX) · [Stellar Lab](https://lab.stellar.org/r/testnet/contract/CD5V5AJCWEP25CVVFQ3J5E54AFOB6JFYO4MIWBJSIPECMEM3PHZ6TCLX) |
| Network | Stellar testnet |
| Source | [`v0.1.1`](https://github.com/WaterUptime/WaterUptime-SmartContract/tree/v0.1.1) (commit `38760fa`) |
| WASM | [`water_uptime_v0.1.1.wasm`](https://github.com/WaterUptime/WaterUptime-SmartContract/releases/tag/v0.1.1_water_uptime_cli28.1.0) · SHA-256 `02c7734bf6457a2dd5500f1a56d034aa9192fbe6b97cc95005d73ab106d4cff8` |
| Build verification | ✅ **Source verified on Stellar Expert**: built in GitHub Actions by [`stellar-expert/soroban-build-workflow`](https://github.com/stellar-expert/soroban-build-workflow) with a [build attestation](https://github.com/WaterUptime/WaterUptime-SmartContract/attestations) (see [`.github/workflows/release.yml`](.github/workflows/release.yml)) |

Check it yourself:

```bash
# 1. The deployed WASM is the attested release asset
stellar contract fetch --id CD5V5AJCWEP25CVVFQ3J5E54AFOB6JFYO4MIWBJSIPECMEM3PHZ6TCLX \
  --network testnet --out-file onchain.wasm
gh release download v0.1.1_water_uptime_cli28.1.0 -R WaterUptime/WaterUptime-SmartContract
sha256sum onchain.wasm water_uptime_v0.1.1.wasm        # hashes should match

# 2. The release asset was built from this repository by the pinned workflow
gh attestation verify water_uptime_v0.1.1.wasm -R WaterUptime/WaterUptime-SmartContract

# 3. The WASM names its source repository
stellar contract info meta --wasm onchain.wasm         # source_repo: github:WaterUptime/WaterUptime-SmartContract
```

Release builds use `stellar contract build --optimize` with Stellar CLI 28.1.0 and source metadata, so a plain local build won't produce the same hash. Verify against the release asset instead.

See [`docs/TESTNET.md`](docs/TESTNET.md) for role addresses, demo transactions, and TypeScript bindings for the frontend.

To deploy your own instance and run a full synthetic job (including a rework cycle):

```bash
scripts/testnet-demo.sh
```

Never use real funds, real water-point identifiers, or personal data.

## Contributing

See the organisation-wide [contributing guide](https://github.com/WaterUptime/.github/blob/main/CONTRIBUTING.md), [security policy](https://github.com/WaterUptime/.github/blob/main/SECURITY.md) and [Code of Conduct](https://github.com/WaterUptime/.github/blob/main/CODE_OF_CONDUCT.md). Report vulnerabilities privately via **Security → Report a vulnerability**. Every PR that changes state transitions or authorisation must include positive and negative tests.

## License

[MIT](LICENSE)
