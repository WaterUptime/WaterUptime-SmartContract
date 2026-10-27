//! Lifecycle events. Payloads carry opaque IDs, addresses and evidence
//! fingerprints only — never personal data. See `docs/CONTRACT_INTERFACE.md`.

use soroban_sdk::{contractevent, Address, BytesN};

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WaterPointRegistered {
    #[topic]
    pub water_point_id: BytesN<32>,
    pub metadata_hash: BytesN<32>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WaterPointStatusChanged {
    #[topic]
    pub water_point_id: BytesN<32>,
    pub active: bool,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReporterUpdated {
    #[topic]
    pub reporter: Address,
    pub allowed: bool,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FaultReported {
    #[topic]
    pub job_id: u64,
    pub water_point_id: BytesN<32>,
    pub reporter: Address,
    pub fault_reference: BytesN<32>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JobAssigned {
    #[topic]
    pub job_id: u64,
    pub technician: Address,
    pub verifier: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepairSubmitted {
    #[topic]
    pub job_id: u64,
    pub technician: Address,
    pub repair_reference: BytesN<32>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepairApproved {
    #[topic]
    pub job_id: u64,
    pub verifier: Address,
    pub verification_reference: BytesN<32>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepairReturned {
    #[topic]
    pub job_id: u64,
    pub verifier: Address,
    pub verification_reference: BytesN<32>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JobClosed {
    #[topic]
    pub job_id: u64,
    pub water_point_id: BytesN<32>,
}
