#![no_std]
//! WaterUptime maintenance-workflow contract.
//!
//! Records an auditable state machine for community water-point repairs:
//! `Reported → Assigned → RepairSubmitted → Verified → Closed`, with
//! `RepairSubmitted → NeedsRework → RepairSubmitted` for rejected repairs.
//! The coordinator can cancel a job that should not proceed (e.g. a mistaken
//! or duplicate report) before repair evidence is awaiting verification.
//!
//! The contract records *authorised claims*; it does not prove that a repair
//! physically happened. Only opaque IDs and evidence fingerprints are stored.

use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, Vec};

mod events;
mod storage;
mod types;

pub use types::{AuditEntry, Error, JobAction, JobStatus, MaintenanceJob, WaterPoint};

use events::{
    FaultReported, JobAssigned, JobCancelled, JobClosed, RepairApproved, RepairReturned,
    RepairSubmitted, ReporterUpdated, WaterPointRegistered, WaterPointStatusChanged,
};

#[contract]
pub struct WaterUptime;

/// Load the stored admin and require its signature.
fn require_admin(env: &Env) -> Result<Address, Error> {
    let admin = storage::get_admin(env)?;
    admin.require_auth();
    Ok(admin)
}

fn load_job(env: &Env, job_id: u64) -> Result<MaintenanceJob, Error> {
    storage::get_job(env, job_id).ok_or(Error::JobNotFound)
}

/// Free the water point once its open job reaches a terminal status.
fn release_water_point(env: &Env, job: &MaintenanceJob) {
    if let Some(mut point) = storage::get_water_point(env, &job.water_point_id) {
        if point.open_job == Some(job.id) {
            point.open_job = None;
            storage::set_water_point(env, &point);
        }
    }
}

/// Fewest audit entries a job in `status` still needs to reach `Closed` or
/// `Cancelled`: a submitted repair needs a decision and then a close or cancel.
fn entries_to_finish(status: JobStatus) -> u32 {
    match status {
        JobStatus::Closed | JobStatus::Cancelled => 0,
        JobStatus::Reported
        | JobStatus::Assigned
        | JobStatus::NeedsRework
        | JobStatus::Verified => 1,
        JobStatus::RepairSubmitted => 2,
    }
}

fn record(
    env: &Env,
    job: &mut MaintenanceJob,
    action: JobAction,
    actor: &Address,
    reference: Option<BytesN<32>>,
) -> Result<(), Error> {
    let now = env.ledger().timestamp();
    job.updated_at = now;
    storage::set_job(env, job);
    storage::append_history(
        env,
        job.id,
        AuditEntry {
            action,
            actor: actor.clone(),
            reference,
            timestamp: now,
        },
        entries_to_finish(job.status),
    )
}

#[contractimpl]
impl WaterUptime {
    /// One-time setup. `admin` acts as the workflow coordinator.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        storage::set_admin(&env, &admin);
        storage::bump_instance(&env);
        Ok(())
    }

    /// Grant or revoke permission to report faults. Revoking does not
    /// affect jobs the reporter already created.
    pub fn set_reporter(env: Env, reporter: Address, allowed: bool) -> Result<(), Error> {
        require_admin(&env)?;
        storage::set_reporter(&env, &reporter, allowed);
        storage::bump_instance(&env);
        ReporterUpdated { reporter, allowed }.publish(&env);
        Ok(())
    }

    pub fn register_water_point(
        env: Env,
        water_point_id: BytesN<32>,
        metadata_hash: BytesN<32>,
    ) -> Result<(), Error> {
        let admin = require_admin(&env)?;
        if storage::get_water_point(&env, &water_point_id).is_some() {
            return Err(Error::WaterPointExists);
        }
        storage::set_water_point(
            &env,
            &WaterPoint {
                id: water_point_id.clone(),
                metadata_hash: metadata_hash.clone(),
                active: true,
                registered_by: admin,
                open_job: None,
            },
        );
        storage::bump_instance(&env);
        WaterPointRegistered {
            water_point_id,
            metadata_hash,
        }
        .publish(&env);
        Ok(())
    }

    /// Activate or deactivate a water point. Inactive points cannot receive
    /// new fault reports; an already-open job continues normally.
    pub fn set_water_point_active(
        env: Env,
        water_point_id: BytesN<32>,
        active: bool,
    ) -> Result<(), Error> {
        require_admin(&env)?;
        let mut point =
            storage::get_water_point(&env, &water_point_id).ok_or(Error::WaterPointNotFound)?;
        point.active = active;
        storage::set_water_point(&env, &point);
        storage::bump_instance(&env);
        WaterPointStatusChanged {
            water_point_id,
            active,
        }
        .publish(&env);
        Ok(())
    }

    /// Open a maintenance job. Only one job may be open per water point.
    pub fn report_fault(
        env: Env,
        reporter: Address,
        water_point_id: BytesN<32>,
        fault_reference: BytesN<32>,
    ) -> Result<u64, Error> {
        reporter.require_auth();
        if !storage::is_reporter(&env, &reporter) {
            return Err(Error::ReporterNotAuthorized);
        }
        let mut point =
            storage::get_water_point(&env, &water_point_id).ok_or(Error::WaterPointNotFound)?;
        if !point.active {
            return Err(Error::WaterPointInactive);
        }
        if point.open_job.is_some() {
            return Err(Error::OpenJobExists);
        }

        let job_id = storage::get_job_count(&env) + 1;
        storage::set_job_count(&env, job_id);
        point.open_job = Some(job_id);
        storage::set_water_point(&env, &point);

        let now = env.ledger().timestamp();
        let mut job = MaintenanceJob {
            id: job_id,
            water_point_id: water_point_id.clone(),
            reporter: reporter.clone(),
            fault_reference: fault_reference.clone(),
            status: JobStatus::Reported,
            technician: None,
            verifier: None,
            repair_reference: None,
            verification_reference: None,
            created_at: now,
            updated_at: now,
        };
        record(
            &env,
            &mut job,
            JobAction::Reported,
            &reporter,
            Some(fault_reference.clone()),
        )?;
        storage::bump_instance(&env);
        FaultReported {
            job_id,
            water_point_id,
            reporter,
            fault_reference,
        }
        .publish(&env);
        Ok(job_id)
    }

    /// Assign (or reassign) a technician and a distinct verifier. Allowed
    /// while the job is `Reported`, `Assigned` or `NeedsRework`.
    pub fn assign_job(
        env: Env,
        job_id: u64,
        technician: Address,
        verifier: Address,
    ) -> Result<(), Error> {
        let admin = require_admin(&env)?;
        if technician == verifier {
            return Err(Error::SameTechnicianAndVerifier);
        }
        let mut job = load_job(&env, job_id)?;
        match job.status {
            JobStatus::Reported | JobStatus::Assigned | JobStatus::NeedsRework => {}
            _ => return Err(Error::InvalidTransition),
        }
        job.status = JobStatus::Assigned;
        job.technician = Some(technician.clone());
        job.verifier = Some(verifier.clone());
        record(&env, &mut job, JobAction::Assigned, &admin, None)?;
        JobAssigned {
            job_id,
            technician,
            verifier,
        }
        .publish(&env);
        Ok(())
    }

    /// Submit a fingerprint of the off-chain repair evidence. Only the
    /// assigned technician may call this, from `Assigned` or `NeedsRework`.
    pub fn submit_repair(
        env: Env,
        technician: Address,
        job_id: u64,
        repair_reference: BytesN<32>,
    ) -> Result<(), Error> {
        technician.require_auth();
        let mut job = load_job(&env, job_id)?;
        if job.technician.as_ref() != Some(&technician) {
            return Err(Error::NotAssignedTechnician);
        }
        match job.status {
            JobStatus::Assigned | JobStatus::NeedsRework => {}
            _ => return Err(Error::InvalidTransition),
        }
        job.status = JobStatus::RepairSubmitted;
        job.repair_reference = Some(repair_reference.clone());
        record(
            &env,
            &mut job,
            JobAction::RepairSubmitted,
            &technician,
            Some(repair_reference.clone()),
        )?;
        RepairSubmitted {
            job_id,
            technician,
            repair_reference,
        }
        .publish(&env);
        Ok(())
    }

    /// Approve the submitted repair (`Verified`) or return it for rework
    /// (`NeedsRework`). Only the assigned verifier may call this.
    pub fn verify_repair(
        env: Env,
        verifier: Address,
        job_id: u64,
        verification_reference: BytesN<32>,
        approved: bool,
    ) -> Result<(), Error> {
        verifier.require_auth();
        let mut job = load_job(&env, job_id)?;
        if job.verifier.as_ref() != Some(&verifier) {
            return Err(Error::NotAssignedVerifier);
        }
        if job.status != JobStatus::RepairSubmitted {
            return Err(Error::InvalidTransition);
        }
        job.verification_reference = Some(verification_reference.clone());
        if approved {
            job.status = JobStatus::Verified;
            record(
                &env,
                &mut job,
                JobAction::RepairApproved,
                &verifier,
                Some(verification_reference.clone()),
            )?;
            RepairApproved {
                job_id,
                verifier,
                verification_reference,
            }
            .publish(&env);
        } else {
            job.status = JobStatus::NeedsRework;
            record(
                &env,
                &mut job,
                JobAction::RepairReturned,
                &verifier,
                Some(verification_reference.clone()),
            )?;
            RepairReturned {
                job_id,
                verifier,
                verification_reference,
            }
            .publish(&env);
        }
        Ok(())
    }

    /// Close a verified job and free the water point for new reports.
    pub fn close_job(env: Env, job_id: u64) -> Result<(), Error> {
        let admin = require_admin(&env)?;
        let mut job = load_job(&env, job_id)?;
        if job.status != JobStatus::Verified {
            return Err(Error::InvalidTransition);
        }
        job.status = JobStatus::Closed;
        record(&env, &mut job, JobAction::Closed, &admin, None)?;
        release_water_point(&env, &job);
        JobClosed {
            job_id,
            water_point_id: job.water_point_id,
        }
        .publish(&env);
        Ok(())
    }

    /// Cancel a job that should not proceed, recording a fingerprint of the
    /// off-chain reason. Allowed from `Reported`, `Assigned` or `NeedsRework`;
    /// a submitted repair must be assessed by the verifier first. Frees the
    /// water point for new reports. History is kept.
    pub fn cancel_job(env: Env, job_id: u64, reason_reference: BytesN<32>) -> Result<(), Error> {
        let admin = require_admin(&env)?;
        let mut job = load_job(&env, job_id)?;
        match job.status {
            JobStatus::Reported | JobStatus::Assigned | JobStatus::NeedsRework => {}
            _ => return Err(Error::InvalidTransition),
        }
        job.status = JobStatus::Cancelled;
        record(
            &env,
            &mut job,
            JobAction::Cancelled,
            &admin,
            Some(reason_reference.clone()),
        )?;
        release_water_point(&env, &job);
        JobCancelled {
            job_id,
            water_point_id: job.water_point_id,
            reason_reference,
        }
        .publish(&env);
        Ok(())
    }

    // ---- Read-only ----

    pub fn get_admin(env: Env) -> Result<Address, Error> {
        storage::get_admin(&env)
    }

    pub fn is_reporter(env: Env, reporter: Address) -> bool {
        storage::is_reporter(&env, &reporter)
    }

    pub fn get_water_point(env: Env, water_point_id: BytesN<32>) -> Result<WaterPoint, Error> {
        storage::get_water_point(&env, &water_point_id).ok_or(Error::WaterPointNotFound)
    }

    pub fn get_job(env: Env, job_id: u64) -> Result<MaintenanceJob, Error> {
        load_job(&env, job_id)
    }

    pub fn get_job_history(env: Env, job_id: u64) -> Result<Vec<AuditEntry>, Error> {
        load_job(&env, job_id)?;
        Ok(storage::get_history(&env, job_id))
    }

    pub fn job_count(env: Env) -> u64 {
        storage::get_job_count(&env)
    }
}

mod test;
