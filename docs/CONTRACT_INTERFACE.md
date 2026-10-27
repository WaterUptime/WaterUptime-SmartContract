# Contract Interface

Interface of the `water_uptime` Soroban contract. Keep this file in sync with `contracts/water_uptime/src/`.

All IDs and references are `BytesN<32>`: either a random opaque ID or a fingerprint (hash) of an off-chain record. Never derive an ID from personal data or exact coordinates.

## Entry points

### Admin / coordinator (stored admin must sign)

| Function | Notes |
|---|---|
| `initialize(admin: Address)` | One-time. `admin` must sign. |
| `set_reporter(reporter: Address, allowed: bool)` | Grant or revoke fault-reporting permission. |
| `register_water_point(water_point_id, metadata_hash)` | Fails with `WaterPointExists` on duplicates. |
| `set_water_point_active(water_point_id, active: bool)` | Inactive points cannot receive new reports. |
| `assign_job(job_id: u64, technician: Address, verifier: Address)` | Technician ≠ verifier. Also used for reassignment. |
| `close_job(job_id: u64)` | Only from `Verified`. Frees the water point for new reports. |
| `cancel_job(job_id: u64, reason_reference)` | Only from `Reported`, `Assigned`, `NeedsRework`. Terminal. Records a fingerprint of the off-chain reason and frees the water point. |

### Role actions (caller must sign)

| Function | Caller |
|---|---|
| `report_fault(reporter, water_point_id, fault_reference) -> u64` | authorised reporter |
| `submit_repair(technician, job_id, repair_reference)` | assigned technician |
| `verify_repair(verifier, job_id, verification_reference, approved: bool)` | assigned verifier |

### Read-only

`get_admin()`, `is_reporter(address)`, `get_water_point(id)`, `get_job(job_id)`, `get_job_history(job_id) -> Vec<AuditEntry>`, `job_count()`.

## Statuses

`Reported`, `Assigned`, `RepairSubmitted`, `NeedsRework`, `Verified`, `Closed`, `Cancelled`. `Closed` and `Cancelled` are terminal. New variants are only ever appended.

## Errors

| Code | Name | Meaning |
|---|---|---|
| 1 | `AlreadyInitialized` | `initialize` called twice |
| 2 | `NotInitialized` | contract not initialised |
| 3 | `WaterPointExists` | water point ID already registered |
| 4 | `WaterPointNotFound` | unknown water point |
| 5 | `WaterPointInactive` | water point deactivated |
| 6 | `ReporterNotAuthorized` | caller is not an authorised reporter |
| 7 | `OpenJobExists` | water point already has an open job |
| 8 | `JobNotFound` | unknown job ID |
| 9 | `InvalidTransition` | action not allowed in the job's current status (including any change to a `Closed` or `Cancelled` job) |
| 10 | `SameTechnicianAndVerifier` | role-separation violation |
| 11 | `NotAssignedTechnician` | caller is not the job's technician |
| 12 | `NotAssignedVerifier` | caller is not the job's verifier |
| 13 | `HistoryLimitReached` | job reached `MAX_HISTORY_LEN` audit entries |

## Events

Each event's first topic is its snake_case name, and its second topic is the indexed field. Data is a map of the remaining fields.

| Event | Indexed topic | Data |
|---|---|---|
| `water_point_registered` | `water_point_id` | `metadata_hash` |
| `water_point_status_changed` | `water_point_id` | `active` |
| `reporter_updated` | `reporter` | `allowed` |
| `fault_reported` | `job_id` | `water_point_id`, `reporter`, `fault_reference` |
| `job_assigned` | `job_id` | `technician`, `verifier` |
| `repair_submitted` | `job_id` | `technician`, `repair_reference` |
| `repair_approved` | `job_id` | `verifier`, `verification_reference` |
| `repair_returned` | `job_id` | `verifier`, `verification_reference` |
| `job_closed` | `job_id` | `water_point_id` |
| `job_cancelled` | `job_id` | `water_point_id`, `reason_reference` |

Payloads contain only addresses, opaque IDs and fingerprints. Never emit personal data.

## Storage

- **Instance:** `Admin`, `JobCount`.
- **Persistent:** `Reporter(Address)`, `WaterPoint(id)`, `Job(u64)`, `History(u64)`.

Persistent entries are extended to ~120 days whenever they are read or written. Records that nobody touches will eventually need a TTL extension or restore. A future issue should add explicit maintenance tooling for this.
