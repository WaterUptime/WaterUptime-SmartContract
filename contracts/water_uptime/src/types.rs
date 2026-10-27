use soroban_sdk::{contracterror, contracttype, Address, BytesN};

/// Maximum number of audit entries kept on-chain for a single job.
///
/// Bounds storage growth when a job loops through rework many times.
pub const MAX_HISTORY_LEN: u32 = 50;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    WaterPointExists = 3,
    WaterPointNotFound = 4,
    WaterPointInactive = 5,
    ReporterNotAuthorized = 6,
    OpenJobExists = 7,
    JobNotFound = 8,
    InvalidTransition = 9,
    SameTechnicianAndVerifier = 10,
    NotAssignedTechnician = 11,
    NotAssignedVerifier = 12,
    HistoryLimitReached = 13,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum JobStatus {
    Reported,
    Assigned,
    RepairSubmitted,
    NeedsRework,
    Verified,
    Closed,
    Cancelled,
}

/// A registered water point. Holds only an opaque ID and a fingerprint of
/// off-chain metadata — never names, coordinates or other personal data.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WaterPoint {
    pub id: BytesN<32>,
    pub metadata_hash: BytesN<32>,
    pub active: bool,
    pub registered_by: Address,
    /// The job currently open for this water point, if any.
    pub open_job: Option<u64>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaintenanceJob {
    pub id: u64,
    pub water_point_id: BytesN<32>,
    pub reporter: Address,
    pub fault_reference: BytesN<32>,
    pub status: JobStatus,
    pub technician: Option<Address>,
    pub verifier: Option<Address>,
    pub repair_reference: Option<BytesN<32>>,
    pub verification_reference: Option<BytesN<32>>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum JobAction {
    Reported,
    Assigned,
    RepairSubmitted,
    RepairApproved,
    RepairReturned,
    Closed,
    Cancelled,
}

/// One entry in a job's append-only audit history.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditEntry {
    pub action: JobAction,
    pub actor: Address,
    pub reference: Option<BytesN<32>>,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    JobCount,
    Reporter(Address),
    WaterPoint(BytesN<32>),
    Job(u64),
    History(u64),
}
