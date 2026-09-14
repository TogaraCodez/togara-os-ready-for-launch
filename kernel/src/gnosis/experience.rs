//! GNOSIS REL-13 — bounded prediction experience and error learning.
//!
//! Experience records what was predicted, what actually happened, and the
//! resulting prediction error. It is derived learning state and never mutates
//! canonical GNOSIS objects.

use super::relation::{ProvenanceSource, RelationProvenance};
use super::semantic::{ContextId, ObjectId, SemanticId};
use spin::Mutex;

pub const SCALE: u32 = 10_000;
pub const MAX_EXPERIENCES: usize = 32;
pub const MAX_WORK: usize = 512;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExperienceRecord {
    pub object_id: ObjectId,
    pub semantic_id: SemanticId,
    pub context: ContextId,
    pub predicted_state: SemanticId,
    pub actual_state: SemanticId,
    pub error_fixed: u32,
    pub confidence_fixed: u32,
    pub support: u32,
    pub timestamp: u64,
    pub provenance: RelationProvenance,
}

impl ExperienceRecord {
    pub const fn empty() -> Self {
        Self {
            object_id: 0,
            semantic_id: 0,
            context: 0,
            predicted_state: 0,
            actual_state: 0,
            error_fixed: SCALE,
            confidence_fixed: 0,
            support: 0,
            timestamp: 0,
            provenance: RelationProvenance {
                origin: 0,
                source: ProvenanceSource::Local,
                revision: 0,
            },
        }
    }
}

#[derive(Clone, Copy)]
struct ExperienceState {
    records: [ExperienceRecord; MAX_EXPERIENCES],
    used: usize,
    cursor: usize,
}

impl ExperienceState {
    const fn empty() -> Self {
        Self {
            records: [ExperienceRecord::empty(); MAX_EXPERIENCES],
            used: 0,
            cursor: 0,
        }
    }
}

static EXPERIENCES: Mutex<ExperienceState> = Mutex::new(ExperienceState::empty());

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExperienceReport {
    pub recorded: usize,
    pub rejected: usize,
    pub work_used: usize,
    pub work_budget: usize,
    pub stopped_by_budget: bool,
    pub latest: ExperienceRecord,
}

impl ExperienceReport {
    pub const fn empty() -> Self {
        Self {
            recorded: 0,
            rejected: 0,
            work_used: 0,
            work_budget: MAX_WORK,
            stopped_by_budget: false,
            latest: ExperienceRecord::empty(),
        }
    }
}

#[inline]
fn state_error(predicted: SemanticId, actual: SemanticId) -> u32 {
    if predicted == actual { 0 } else { SCALE }
}

#[inline]
pub fn observe(
    object_id: ObjectId,
    semantic_id: SemanticId,
    context: ContextId,
    predicted_state: SemanticId,
    actual_state: SemanticId,
    confidence_fixed: u32,
    support: u32,
    timestamp: u64,
    provenance: RelationProvenance,
) -> Option<ExperienceRecord> {
    if object_id == 0
        || !super::semantic::is_valid(semantic_id)
        || !super::semantic::is_valid(predicted_state)
        || !super::semantic::is_valid(actual_state)
        || provenance.revision == 0
        || !matches!(
            provenance.source,
            ProvenanceSource::Local | ProvenanceSource::Remote | ProvenanceSource::Recovered
        )
    {
        return None;
    }

    let record = ExperienceRecord {
        object_id,
        semantic_id,
        context,
        predicted_state,
        actual_state,
        error_fixed: state_error(predicted_state, actual_state),
        confidence_fixed: confidence_fixed.min(SCALE),
        support,
        timestamp,
        provenance,
    };

    let mut state = EXPERIENCES.lock();
    let index = state.cursor % MAX_EXPERIENCES;
    state.records[index] = record;

    if state.used < MAX_EXPERIENCES {
        state.used += 1;
    }
    state.cursor = state.cursor.wrapping_add(1);

    Some(record)
}

#[inline]
pub fn count() -> usize {
    EXPERIENCES.lock().used
}

#[inline]
pub fn latest() -> ExperienceRecord {
    let state = EXPERIENCES.lock();
    if state.used == 0 {
        return ExperienceRecord::empty();
    }
    state.records[(state.cursor + MAX_EXPERIENCES - 1) % MAX_EXPERIENCES]
}

#[inline]
pub fn mean_error_fixed() -> u32 {
    let state = EXPERIENCES.lock();
    if state.used == 0 {
        return 0;
    }

    let mut total = 0u64;
    let mut index = 0usize;
    while index < state.used {
        total += state.records[index].error_fixed as u64;
        index += 1;
    }

    (total / state.used as u64) as u32
}

#[inline]
pub fn clear() {
    let mut state = EXPERIENCES.lock();
    *state = ExperienceState::empty();
}

#[inline]
pub fn for_each(mut callback: impl FnMut(&ExperienceRecord)) {
    let mut snapshot = [ExperienceRecord::empty(); MAX_EXPERIENCES];
    let used;

    {
        let state = EXPERIENCES.lock();
        used = state.used;
        let mut index = 0usize;
        while index < used {
            snapshot[index] = state.records[index];
            index += 1;
        }
    }

    let mut index = 0usize;
    while index < used {
        callback(&snapshot[index]);
        index += 1;
    }
}

#[inline]
pub fn self_test() -> bool {
    clear();

    let provenance = RelationProvenance {
        origin: 1,
        source: ProvenanceSource::Local,
        revision: 1,
    };

    let first = observe(1, 2, 3, 10, 10, 9_000, 3, 100, provenance);
    let second = observe(1, 2, 3, 10, 20, 9_000, 3, 101, provenance);

    if first.is_none() || second.is_none() || count() != 2 {
        clear();
        return false;
    }

    if latest().error_fixed != SCALE || mean_error_fixed() != SCALE / 2 {
        clear();
        return false;
    }

    let mut seen = 0usize;
    for_each(|_| {
        seen += 1;
        let _ = count();
    });

    let ok = seen == 2;
    clear();
    ok
}
