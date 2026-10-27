#![cfg(test)]
extern crate std;

use super::*;
use crate::events::{FaultReported, JobCancelled, JobClosed, RepairReturned};
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    xdr::{ContractEventBody, ScSymbol, ScVal},
    BytesN, Env, Event,
};

struct Setup<'a> {
    env: Env,
    client: WaterUptimeClient<'a>,
    contract_id: Address,
    admin: Address,
    reporter: Address,
    technician: Address,
    verifier: Address,
    point: BytesN<32>,
}

fn hash(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

/// Initialised contract with one registered water point and one reporter.
fn setup<'a>() -> Setup<'a> {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let contract_id = env.register(WaterUptime, ());
    let client = WaterUptimeClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let reporter = Address::generate(&env);
    let technician = Address::generate(&env);
    let verifier = Address::generate(&env);
    let point = hash(&env, 1);

    client.initialize(&admin);
    client.set_reporter(&reporter, &true);
    client.register_water_point(&point, &hash(&env, 2));

    Setup {
        env,
        client,
        contract_id,
        admin,
        reporter,
        technician,
        verifier,
        point,
    }
}

impl Setup<'_> {
    fn report(&self) -> u64 {
        self.client
            .report_fault(&self.reporter, &self.point, &hash(&self.env, 10))
    }

    fn assigned_job(&self) -> u64 {
        let id = self.report();
        self.client
            .assign_job(&id, &self.technician, &self.verifier);
        id
    }

    fn submitted_job(&self) -> u64 {
        let id = self.assigned_job();
        self.client
            .submit_repair(&self.technician, &id, &hash(&self.env, 20));
        id
    }

    fn verified_job(&self) -> u64 {
        let id = self.submitted_job();
        self.client
            .verify_repair(&self.verifier, &id, &hash(&self.env, 30), &true);
        id
    }

    fn status(&self, id: u64) -> JobStatus {
        self.client.get_job(&id).status
    }

    /// Address that signed the most recent top-level invocation.
    fn last_signer(&self) -> Address {
        self.env.auths().last().unwrap().0.clone()
    }
}

// ---- Initialisation ----

#[test]
fn initialize_sets_admin_once() {
    let s = setup();
    assert_eq!(s.client.get_admin(), s.admin);
    assert_eq!(
        s.client.try_initialize(&Address::generate(&s.env)),
        Err(Ok(Error::AlreadyInitialized))
    );
}

#[test]
fn calls_before_initialize_fail() {
    let env = Env::default();
    env.mock_all_auths();
    let client = WaterUptimeClient::new(&env, &env.register(WaterUptime, ()));
    assert_eq!(
        client.try_register_water_point(&hash(&env, 1), &hash(&env, 2)),
        Err(Ok(Error::NotInitialized))
    );
    assert_eq!(client.try_get_admin(), Err(Ok(Error::NotInitialized)));
}

// ---- Happy path ----

#[test]
fn full_lifecycle_report_to_close() {
    let s = setup();
    let id = s.report();
    assert_eq!(id, 1);
    assert_eq!(s.client.job_count(), 1);
    assert_eq!(s.status(id), JobStatus::Reported);
    assert_eq!(s.client.get_water_point(&s.point).open_job, Some(id));

    s.client.assign_job(&id, &s.technician, &s.verifier);
    assert_eq!(s.status(id), JobStatus::Assigned);

    s.env.ledger().set_timestamp(2_000);
    s.client
        .submit_repair(&s.technician, &id, &hash(&s.env, 20));
    assert_eq!(s.status(id), JobStatus::RepairSubmitted);

    s.client
        .verify_repair(&s.verifier, &id, &hash(&s.env, 30), &true);
    assert_eq!(s.status(id), JobStatus::Verified);

    s.client.close_job(&id);
    let job = s.client.get_job(&id);
    assert_eq!(job.status, JobStatus::Closed);
    assert_eq!(job.technician, Some(s.technician.clone()));
    assert_eq!(job.verifier, Some(s.verifier.clone()));
    assert_eq!(job.repair_reference, Some(hash(&s.env, 20)));
    assert_eq!(job.verification_reference, Some(hash(&s.env, 30)));
    assert_eq!(job.created_at, 1_000);
    assert_eq!(job.updated_at, 2_000);
    assert_eq!(s.client.get_water_point(&s.point).open_job, None);

    let actions: std::vec::Vec<JobAction> = s
        .client
        .get_job_history(&id)
        .iter()
        .map(|e| e.action)
        .collect();
    assert_eq!(
        actions,
        [
            JobAction::Reported,
            JobAction::Assigned,
            JobAction::RepairSubmitted,
            JobAction::RepairApproved,
            JobAction::Closed,
        ]
    );
}

#[test]
fn rework_cycle_preserves_history() {
    let s = setup();
    let id = s.submitted_job();

    s.client
        .verify_repair(&s.verifier, &id, &hash(&s.env, 31), &false);
    assert_eq!(s.status(id), JobStatus::NeedsRework);
    assert_eq!(
        s.client.try_close_job(&id),
        Err(Ok(Error::InvalidTransition))
    );

    s.client
        .submit_repair(&s.technician, &id, &hash(&s.env, 21));
    s.client
        .verify_repair(&s.verifier, &id, &hash(&s.env, 32), &true);
    s.client.close_job(&id);

    let history = s.client.get_job_history(&id);
    let actions: std::vec::Vec<JobAction> = history.iter().map(|e| e.action).collect();
    assert_eq!(
        actions,
        [
            JobAction::Reported,
            JobAction::Assigned,
            JobAction::RepairSubmitted,
            JobAction::RepairReturned,
            JobAction::RepairSubmitted,
            JobAction::RepairApproved,
            JobAction::Closed,
        ]
    );
    // The first, rejected evidence reference is still on record.
    assert_eq!(history.get(2).unwrap().reference, Some(hash(&s.env, 20)));
    assert_eq!(history.get(3).unwrap().actor, s.verifier);
}

#[test]
fn water_point_accepts_new_report_after_close() {
    let s = setup();
    let first = s.verified_job();
    s.client.close_job(&first);
    let second = s.report();
    assert_eq!(second, 2);
    assert_eq!(s.client.get_water_point(&s.point).open_job, Some(second));
}

#[test]
fn reassignment_before_submission() {
    let s = setup();
    let id = s.assigned_job();
    let new_tech = Address::generate(&s.env);
    s.client.assign_job(&id, &new_tech, &s.verifier);

    assert_eq!(
        s.client
            .try_submit_repair(&s.technician, &id, &hash(&s.env, 20)),
        Err(Ok(Error::NotAssignedTechnician))
    );
    s.client.submit_repair(&new_tech, &id, &hash(&s.env, 20));
    assert_eq!(s.status(id), JobStatus::RepairSubmitted);
}

// ---- Authorisation ----

#[test]
fn admin_actions_require_admin_signature() {
    let s = setup();
    s.client
        .register_water_point(&hash(&s.env, 3), &hash(&s.env, 4));
    assert_eq!(s.last_signer(), s.admin);

    let id = s.report();
    s.client.assign_job(&id, &s.technician, &s.verifier);
    assert_eq!(s.last_signer(), s.admin);
}

#[test]
fn role_actions_require_caller_signature() {
    let s = setup();
    let id = s.report();
    assert_eq!(s.last_signer(), s.reporter);
    s.client.assign_job(&id, &s.technician, &s.verifier);
    s.client
        .submit_repair(&s.technician, &id, &hash(&s.env, 20));
    assert_eq!(s.last_signer(), s.technician);
    s.client
        .verify_repair(&s.verifier, &id, &hash(&s.env, 30), &true);
    assert_eq!(s.last_signer(), s.verifier);
}

#[test]
fn unsigned_admin_call_is_rejected() {
    let env = Env::default();
    let client = WaterUptimeClient::new(&env, &env.register(WaterUptime, ()));
    let admin = Address::generate(&env);
    env.mock_all_auths();
    client.initialize(&admin);

    // Drop the blanket auth mock: no signature is supplied.
    env.set_auths(&[]);
    assert!(client
        .try_register_water_point(&hash(&env, 1), &hash(&env, 2))
        .is_err());
}

#[test]
fn unauthorised_reporter_cannot_report() {
    let s = setup();
    let stranger = Address::generate(&s.env);
    assert_eq!(
        s.client
            .try_report_fault(&stranger, &s.point, &hash(&s.env, 10)),
        Err(Ok(Error::ReporterNotAuthorized))
    );
    assert_eq!(s.client.job_count(), 0);
}

#[test]
fn revoked_reporter_keeps_existing_jobs() {
    let s = setup();
    let id = s.report();
    s.client.set_reporter(&s.reporter, &false);
    assert!(!s.client.is_reporter(&s.reporter));

    let other_point = hash(&s.env, 5);
    s.client
        .register_water_point(&other_point, &hash(&s.env, 6));
    assert_eq!(
        s.client
            .try_report_fault(&s.reporter, &other_point, &hash(&s.env, 11)),
        Err(Ok(Error::ReporterNotAuthorized))
    );
    assert_eq!(s.client.get_job(&id).reporter, s.reporter);
}

#[test]
fn only_assigned_technician_can_submit() {
    let s = setup();
    let id = s.assigned_job();
    assert_eq!(
        s.client
            .try_submit_repair(&s.verifier, &id, &hash(&s.env, 20)),
        Err(Ok(Error::NotAssignedTechnician))
    );
    assert_eq!(s.status(id), JobStatus::Assigned);
}

#[test]
fn only_assigned_verifier_can_verify() {
    let s = setup();
    let id = s.submitted_job();
    assert_eq!(
        s.client
            .try_verify_repair(&s.technician, &id, &hash(&s.env, 30), &true),
        Err(Ok(Error::NotAssignedVerifier))
    );
    assert_eq!(
        s.client
            .try_verify_repair(&Address::generate(&s.env), &id, &hash(&s.env, 30), &true),
        Err(Ok(Error::NotAssignedVerifier))
    );
    assert_eq!(s.status(id), JobStatus::RepairSubmitted);
}

// ---- Role separation ----

#[test]
fn technician_and_verifier_must_differ() {
    let s = setup();
    let id = s.report();
    let before = s.client.get_job(&id);
    assert_eq!(
        s.client.try_assign_job(&id, &s.technician, &s.technician),
        Err(Ok(Error::SameTechnicianAndVerifier))
    );
    assert_eq!(s.client.get_job(&id), before);
    assert_eq!(s.client.get_job_history(&id).len(), 1);
}

// ---- Water-point rules ----

#[test]
fn duplicate_water_point_rejected() {
    let s = setup();
    assert_eq!(
        s.client
            .try_register_water_point(&s.point, &hash(&s.env, 9)),
        Err(Ok(Error::WaterPointExists))
    );
}

#[test]
fn report_requires_registered_active_point() {
    let s = setup();
    assert_eq!(
        s.client
            .try_report_fault(&s.reporter, &hash(&s.env, 99), &hash(&s.env, 10)),
        Err(Ok(Error::WaterPointNotFound))
    );

    s.client.set_water_point_active(&s.point, &false);
    assert_eq!(
        s.client
            .try_report_fault(&s.reporter, &s.point, &hash(&s.env, 10)),
        Err(Ok(Error::WaterPointInactive))
    );

    s.client.set_water_point_active(&s.point, &true);
    s.report();
}

#[test]
fn duplicate_open_report_rejected() {
    let s = setup();
    s.report();
    assert_eq!(
        s.client
            .try_report_fault(&s.reporter, &s.point, &hash(&s.env, 11)),
        Err(Ok(Error::OpenJobExists))
    );
    assert_eq!(s.client.job_count(), 1);
}

// ---- Invalid / out-of-order transitions ----

#[test]
fn unknown_job_rejected() {
    let s = setup();
    assert_eq!(s.client.try_get_job(&42), Err(Ok(Error::JobNotFound)));
    assert_eq!(
        s.client.try_assign_job(&42, &s.technician, &s.verifier),
        Err(Ok(Error::JobNotFound))
    );
    assert_eq!(s.client.try_close_job(&42), Err(Ok(Error::JobNotFound)));
}

#[test]
fn submit_before_assignment_rejected() {
    let s = setup();
    let id = s.report();
    assert_eq!(
        s.client
            .try_submit_repair(&s.technician, &id, &hash(&s.env, 20)),
        Err(Ok(Error::NotAssignedTechnician))
    );
}

#[test]
fn duplicate_submission_rejected() {
    let s = setup();
    let id = s.submitted_job();
    assert_eq!(
        s.client
            .try_submit_repair(&s.technician, &id, &hash(&s.env, 21)),
        Err(Ok(Error::InvalidTransition))
    );
    assert_eq!(
        s.client.get_job(&id).repair_reference,
        Some(hash(&s.env, 20))
    );
}

#[test]
fn verify_before_submission_rejected() {
    let s = setup();
    let id = s.assigned_job();
    assert_eq!(
        s.client
            .try_verify_repair(&s.verifier, &id, &hash(&s.env, 30), &true),
        Err(Ok(Error::InvalidTransition))
    );
}

#[test]
fn double_verification_rejected() {
    let s = setup();
    let id = s.verified_job();
    assert_eq!(
        s.client
            .try_verify_repair(&s.verifier, &id, &hash(&s.env, 30), &false),
        Err(Ok(Error::InvalidTransition))
    );
    assert_eq!(s.status(id), JobStatus::Verified);
}

#[test]
fn close_before_verification_rejected() {
    let s = setup();
    let reported = s.report();
    assert_eq!(
        s.client.try_close_job(&reported),
        Err(Ok(Error::InvalidTransition))
    );
    s.client.assign_job(&reported, &s.technician, &s.verifier);
    assert_eq!(
        s.client.try_close_job(&reported),
        Err(Ok(Error::InvalidTransition))
    );
    s.client
        .submit_repair(&s.technician, &reported, &hash(&s.env, 20));
    assert_eq!(
        s.client.try_close_job(&reported),
        Err(Ok(Error::InvalidTransition))
    );
}

#[test]
fn closed_job_is_immutable() {
    let s = setup();
    let id = s.verified_job();
    s.client.close_job(&id);

    assert_eq!(
        s.client.try_close_job(&id),
        Err(Ok(Error::InvalidTransition))
    );
    assert_eq!(
        s.client.try_assign_job(&id, &s.technician, &s.verifier),
        Err(Ok(Error::InvalidTransition))
    );
    assert_eq!(
        s.client
            .try_submit_repair(&s.technician, &id, &hash(&s.env, 22)),
        Err(Ok(Error::InvalidTransition))
    );
    assert_eq!(s.client.get_job_history(&id).len(), 5);
}

#[test]
fn reassignment_after_submission_rejected() {
    let s = setup();
    let id = s.submitted_job();
    assert_eq!(
        s.client
            .try_assign_job(&id, &Address::generate(&s.env), &s.verifier),
        Err(Ok(Error::InvalidTransition))
    );
}

#[test]
fn history_is_bounded() {
    let s = setup();
    let id = s.assigned_job();
    // 2 entries so far; each rework loop adds 2 more.
    let loops = (types::MAX_HISTORY_LEN - 2) / 2;
    for _ in 0..loops {
        s.client
            .submit_repair(&s.technician, &id, &hash(&s.env, 20));
        s.client
            .verify_repair(&s.verifier, &id, &hash(&s.env, 30), &false);
    }
    assert_eq!(s.client.get_job_history(&id).len(), types::MAX_HISTORY_LEN);
    assert_eq!(
        s.client
            .try_submit_repair(&s.technician, &id, &hash(&s.env, 20)),
        Err(Ok(Error::HistoryLimitReached))
    );
    // The failed call was rolled back.
    assert_eq!(s.status(id), JobStatus::NeedsRework);
}

// ---- Cancellation ----

#[test]
fn cancel_frees_water_point_and_keeps_history() {
    let s = setup();
    let id = s.assigned_job();
    s.client.cancel_job(&id, &hash(&s.env, 40));
    assert_eq!(s.last_signer(), s.admin);
    assert_eq!(s.status(id), JobStatus::Cancelled);
    assert_eq!(s.client.get_water_point(&s.point).open_job, None);

    let history = s.client.get_job_history(&id);
    let actions: std::vec::Vec<JobAction> = history.iter().map(|e| e.action).collect();
    assert_eq!(
        actions,
        [
            JobAction::Reported,
            JobAction::Assigned,
            JobAction::Cancelled
        ]
    );
    let last = history.last().unwrap();
    assert_eq!(last.actor, s.admin);
    assert_eq!(last.reference, Some(hash(&s.env, 40)));

    // The water point accepts a fresh report.
    let next = s.report();
    assert_eq!(next, 2);
    assert_eq!(s.client.get_water_point(&s.point).open_job, Some(next));
}

#[test]
fn cancel_allowed_from_reported_and_needs_rework() {
    let s = setup();
    let reported = s.report();
    s.client.cancel_job(&reported, &hash(&s.env, 40));
    assert_eq!(s.status(reported), JobStatus::Cancelled);

    let rework = s.submitted_job();
    s.client
        .verify_repair(&s.verifier, &rework, &hash(&s.env, 31), &false);
    s.client.cancel_job(&rework, &hash(&s.env, 41));
    assert_eq!(s.status(rework), JobStatus::Cancelled);
}

#[test]
fn cancel_rejected_once_evidence_submitted_or_terminal() {
    let s = setup();
    let id = s.submitted_job();
    assert_eq!(
        s.client.try_cancel_job(&id, &hash(&s.env, 40)),
        Err(Ok(Error::InvalidTransition))
    );

    s.client
        .verify_repair(&s.verifier, &id, &hash(&s.env, 30), &true);
    assert_eq!(
        s.client.try_cancel_job(&id, &hash(&s.env, 40)),
        Err(Ok(Error::InvalidTransition))
    );

    s.client.close_job(&id);
    assert_eq!(
        s.client.try_cancel_job(&id, &hash(&s.env, 40)),
        Err(Ok(Error::InvalidTransition))
    );
    assert_eq!(
        s.client.try_cancel_job(&42, &hash(&s.env, 40)),
        Err(Ok(Error::JobNotFound))
    );
}

#[test]
fn cancelled_job_is_terminal() {
    let s = setup();
    let id = s.assigned_job();
    s.client.cancel_job(&id, &hash(&s.env, 40));

    assert_eq!(
        s.client.try_cancel_job(&id, &hash(&s.env, 41)),
        Err(Ok(Error::InvalidTransition))
    );
    assert_eq!(
        s.client.try_assign_job(&id, &s.technician, &s.verifier),
        Err(Ok(Error::InvalidTransition))
    );
    assert_eq!(
        s.client
            .try_submit_repair(&s.technician, &id, &hash(&s.env, 20)),
        Err(Ok(Error::InvalidTransition))
    );
    assert_eq!(
        s.client
            .try_verify_repair(&s.verifier, &id, &hash(&s.env, 30), &true),
        Err(Ok(Error::InvalidTransition))
    );
    assert_eq!(
        s.client.try_close_job(&id),
        Err(Ok(Error::InvalidTransition))
    );
    assert_eq!(s.client.get_job_history(&id).len(), 3);
}

#[test]
fn unsigned_cancel_is_rejected() {
    let s = setup();
    let id = s.report();
    s.env.set_auths(&[]);
    assert!(s.client.try_cancel_job(&id, &hash(&s.env, 40)).is_err());
    s.env.mock_all_auths();
    assert_eq!(s.status(id), JobStatus::Reported);
}

#[test]
fn cancel_emits_event() {
    let s = setup();
    let id = s.report();
    s.client.cancel_job(&id, &hash(&s.env, 40));
    assert_eq!(
        s.env.events().all(),
        [JobCancelled {
            job_id: id,
            water_point_id: s.point.clone(),
            reason_reference: hash(&s.env, 40),
        }
        .to_xdr(&s.env, &s.contract_id)]
    );
}

// ---- Events ----

#[test]
fn fault_report_emits_event() {
    let s = setup();
    let id = s.report();
    let expected = FaultReported {
        job_id: id,
        water_point_id: s.point.clone(),
        reporter: s.reporter.clone(),
        fault_reference: hash(&s.env, 10),
    };
    assert_eq!(
        s.env.events().all(),
        [expected.to_xdr(&s.env, &s.contract_id)]
    );
}

#[test]
fn rejection_and_close_emit_events() {
    let s = setup();
    let id = s.submitted_job();
    s.client
        .verify_repair(&s.verifier, &id, &hash(&s.env, 31), &false);
    assert_eq!(
        s.env.events().all(),
        [RepairReturned {
            job_id: id,
            verifier: s.verifier.clone(),
            verification_reference: hash(&s.env, 31),
        }
        .to_xdr(&s.env, &s.contract_id)]
    );

    s.client
        .submit_repair(&s.technician, &id, &hash(&s.env, 21));
    s.client
        .verify_repair(&s.verifier, &id, &hash(&s.env, 32), &true);
    s.client.close_job(&id);
    assert_eq!(
        s.env.events().all(),
        [JobClosed {
            job_id: id,
            water_point_id: s.point.clone(),
        }
        .to_xdr(&s.env, &s.contract_id)]
    );
}

#[test]
fn event_topics_use_documented_names() {
    let s = setup();
    s.report();
    let events = s.env.events().all();
    let event = &events.events()[0];
    let ContractEventBody::V0(body) = &event.body;
    assert_eq!(
        body.topics[0],
        ScVal::Symbol(ScSymbol("fault_reported".try_into().unwrap()))
    );
}
