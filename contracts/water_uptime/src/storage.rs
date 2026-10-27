use soroban_sdk::{Address, BytesN, Env, Vec};

use crate::types::{AuditEntry, DataKey, Error, MaintenanceJob, WaterPoint, MAX_HISTORY_LEN};

const DAY_IN_LEDGERS: u32 = 17_280;
const INSTANCE_BUMP: u32 = 30 * DAY_IN_LEDGERS;
const INSTANCE_THRESHOLD: u32 = INSTANCE_BUMP - DAY_IN_LEDGERS;
const PERSISTENT_BUMP: u32 = 120 * DAY_IN_LEDGERS;
const PERSISTENT_THRESHOLD: u32 = PERSISTENT_BUMP - 7 * DAY_IN_LEDGERS;

pub fn bump_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_THRESHOLD, INSTANCE_BUMP);
}

fn bump_persistent(env: &Env, key: &DataKey) {
    env.storage()
        .persistent()
        .extend_ttl(key, PERSISTENT_THRESHOLD, PERSISTENT_BUMP);
}

pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

pub fn get_admin(env: &Env) -> Result<Address, Error> {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::NotInitialized)
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

pub fn get_job_count(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::JobCount)
        .unwrap_or(0)
}

pub fn set_job_count(env: &Env, count: u64) {
    env.storage().instance().set(&DataKey::JobCount, &count);
}

pub fn is_reporter(env: &Env, reporter: &Address) -> bool {
    let key = DataKey::Reporter(reporter.clone());
    let allowed = env.storage().persistent().has(&key);
    if allowed {
        bump_persistent(env, &key);
    }
    allowed
}

pub fn set_reporter(env: &Env, reporter: &Address, allowed: bool) {
    let key = DataKey::Reporter(reporter.clone());
    if allowed {
        env.storage().persistent().set(&key, &true);
        bump_persistent(env, &key);
    } else {
        env.storage().persistent().remove(&key);
    }
}

pub fn get_water_point(env: &Env, id: &BytesN<32>) -> Option<WaterPoint> {
    let key = DataKey::WaterPoint(id.clone());
    let point = env.storage().persistent().get(&key);
    if point.is_some() {
        bump_persistent(env, &key);
    }
    point
}

pub fn set_water_point(env: &Env, point: &WaterPoint) {
    let key = DataKey::WaterPoint(point.id.clone());
    env.storage().persistent().set(&key, point);
    bump_persistent(env, &key);
}

pub fn get_job(env: &Env, job_id: u64) -> Option<MaintenanceJob> {
    let key = DataKey::Job(job_id);
    let job = env.storage().persistent().get(&key);
    if job.is_some() {
        bump_persistent(env, &key);
    }
    job
}

pub fn set_job(env: &Env, job: &MaintenanceJob) {
    let key = DataKey::Job(job.id);
    env.storage().persistent().set(&key, job);
    bump_persistent(env, &key);
}

pub fn get_history(env: &Env, job_id: u64) -> Vec<AuditEntry> {
    let key = DataKey::History(job_id);
    match env.storage().persistent().get(&key) {
        Some(history) => {
            bump_persistent(env, &key);
            history
        }
        None => Vec::new(env),
    }
}

pub fn append_history(env: &Env, job_id: u64, entry: AuditEntry) -> Result<(), Error> {
    let mut history = get_history(env, job_id);
    if history.len() >= MAX_HISTORY_LEN {
        return Err(Error::HistoryLimitReached);
    }
    history.push_back(entry);
    let key = DataKey::History(job_id);
    env.storage().persistent().set(&key, &history);
    bump_persistent(env, &key);
    Ok(())
}
