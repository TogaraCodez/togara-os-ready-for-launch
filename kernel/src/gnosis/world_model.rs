//! GNOSIS bounded World Model.
//!
//! Maintains a derived world snapshot and explicit one-step predictions.
//! Canonical KOBJ state remains authoritative.

use super::delta::SemanticDelta;
use super::experience::ExperienceRecord;
use super::relations::{ProvenanceSource, RelationProvenance};
use super::semantic::{ContextId, ObjectId, SemanticId};
use spin::Mutex;

pub const MAX_WORLD_OBJECTS: usize = 32;
pub const MAX_WORK: usize = 256;
pub const SCALE: u32 = 10_000;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldFact {
    pub object_id: ObjectId,
    pub semantic_id: SemanticId,
    pub state: SemanticId,
    pub context: ContextId,
    pub confidence_fixed: u32,
    pub timestamp: u64,
}
impl WorldFact {
    pub const fn empty() -> Self {
        Self {
            object_id: 0,
            semantic_id: 0,
            state: 0,
            context: 0,
            confidence_fixed: 0,
            timestamp: 0,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldPrediction {
    pub object_id: ObjectId,
    pub semantic_id: SemanticId,
    pub predicted_state: SemanticId,
    pub confidence_fixed: u32,
    pub timestamp: u64,
    pub provenance: RelationProvenance,
}
impl WorldPrediction {
    pub const fn empty() -> Self {
        Self {
            object_id: 0,
            semantic_id: 0,
            predicted_state: 0,
            confidence_fixed: 0,
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
struct State {
    facts: [WorldFact; MAX_WORLD_OBJECTS],
    used: usize,
    predictions: [WorldPrediction; MAX_WORLD_OBJECTS],
    predicted: usize,
}
impl State {
    const fn empty() -> Self {
        Self {
            facts: [WorldFact::empty(); MAX_WORLD_OBJECTS],
            used: 0,
            predictions: [WorldPrediction::empty(); MAX_WORLD_OBJECTS],
            predicted: 0,
        }
    }
}
static WORLD: Mutex<State> = Mutex::new(State::empty());
fn valid_p(p: &RelationProvenance) -> bool {
    p.revision != 0
        && matches!(
            p.source,
            ProvenanceSource::Local | ProvenanceSource::Remote | ProvenanceSource::Recovered
        )
}
pub fn clear() {
    *WORLD.lock() = State::empty()
}
pub fn observe_delta(delta: SemanticDelta) -> bool {
    if !delta.is_valid() {
        return false;
    }
    let mut s = WORLD.lock();
    let mut i = 0;
    while i < s.used {
        if s.facts[i].object_id == delta.object_id && s.facts[i].semantic_id == delta.semantic_id {
            s.facts[i].state = delta.new_state;
            s.facts[i].context = delta.context;
            s.facts[i].timestamp = delta.timestamp;
            s.facts[i].confidence_fixed = SCALE;
            return true;
        }
        i += 1
    }
    if s.used >= MAX_WORLD_OBJECTS {
        return false;
    }
    let index = s.used;
    s.facts[index] = WorldFact {
        object_id: delta.object_id,
        semantic_id: delta.semantic_id,
        state: delta.new_state,
        context: delta.context,
        confidence_fixed: SCALE,
        timestamp: delta.timestamp,
    };
    s.used = index + 1;
    true
}
pub fn predict(
    object_id: ObjectId,
    semantic_id: SemanticId,
    state: SemanticId,
    confidence_fixed: u32,
    timestamp: u64,
    provenance: RelationProvenance,
) -> bool {
    if object_id == 0
        || !super::semantic::is_valid(semantic_id)
        || !super::semantic::is_valid(state)
        || !valid_p(&provenance)
    {
        return false;
    }
    let mut s = WORLD.lock();
    if s.predicted >= MAX_WORLD_OBJECTS {
        return false;
    }
    let index = s.predicted;
    s.predictions[index] = WorldPrediction {
        object_id,
        semantic_id,
        predicted_state: state,
        confidence_fixed: confidence_fixed.min(SCALE),
        timestamp,
        provenance,
    };
    s.predicted = index + 1;
    true
}
pub fn prediction_error(actual: &ExperienceRecord) -> u32 {
    if actual.predicted_state == actual.actual_state {
        0
    } else {
        SCALE
    }
}
pub fn fact(object_id: ObjectId) -> Option<WorldFact> {
    let s = WORLD.lock();
    let mut i = 0;
    while i < s.used {
        if s.facts[i].object_id == object_id {
            return Some(s.facts[i]);
        }
        i += 1
    }
    None
}
pub fn count() -> usize {
    WORLD.lock().used
}
pub fn self_test() -> bool {
    clear();
    let p = RelationProvenance {
        origin: 1,
        source: ProvenanceSource::Local,
        revision: 1,
    };
    let d = SemanticDelta {
        object_id: 1,
        semantic_id: super::semantic::STATE_OBJECT,
        previous_state: super::semantic::READY,
        new_state: super::semantic::ACTIVE,
        context: 2,
        timestamp: 3,
        provenance: p,
    };
    if !observe_delta(d) {
        return false;
    }
    if fact(1).map(|v| v.state) != Some(super::semantic::ACTIVE) {
        return false;
    }
    if !predict(
        1,
        super::semantic::STATE_OBJECT,
        super::semantic::COMPLETE,
        8000,
        4,
        p,
    ) {
        return false;
    }
    clear();
    count() == 0
}
