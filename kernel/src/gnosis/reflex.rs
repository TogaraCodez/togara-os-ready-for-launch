//! GNOSIS semantic reflex observer.
//!
//! REL-05 is intentionally observation-only. It identifies a specific semantic
//! transition already present in an attention record without mutating queue
//! state, canonical GNOSIS registries, or any action layer.

use super::attention::AttentionRecord;
use super::relation::RelationProvenance;
use super::semantic::{ContextId, ObjectId, SemanticId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReflexTrigger {
    pub object_id: ObjectId,
    pub semantic_id: SemanticId,
    pub previous_state: SemanticId,
    pub new_state: SemanticId,
    pub context: ContextId,
    pub timestamp: u64,
    pub provenance: RelationProvenance,
}

#[inline]
fn matches_reflex_condition(record: &AttentionRecord) -> bool {
    record.semantic_id == super::semantic::AUTHORIZATION
        && record.previous_state == super::semantic::READY
        && record.new_state == super::semantic::FAILED
}

#[inline]
pub fn evaluate(record: &AttentionRecord) -> Option<ReflexTrigger> {
    if !matches_reflex_condition(record) {
        return None;
    }

    Some(ReflexTrigger {
        object_id: record.object_id,
        semantic_id: record.semantic_id,
        previous_state: record.previous_state,
        new_state: record.new_state,
        context: record.context,
        timestamp: record.timestamp,
        provenance: record.provenance,
    })
}

#[inline]
pub fn observe() -> Option<ReflexTrigger> {
    let mut candidate = None;

    super::attention::for_each(|record| {
        if candidate.is_none() && matches_reflex_condition(record) {
            candidate = Some(ReflexTrigger {
                object_id: record.object_id,
                semantic_id: record.semantic_id,
                previous_state: record.previous_state,
                new_state: record.new_state,
                context: record.context,
                timestamp: record.timestamp,
                provenance: record.provenance,
            });
        }
    });

    candidate
}

#[inline]
pub fn self_test() -> bool {
    super::attention::clear();
    let queue_before = super::attention::count();
    let delta_before = super::delta::count();

    if queue_before != 0 || delta_before != 0 {
        return false;
    }

    let matching_delta = super::delta::SemanticDelta {
        object_id: 77,
        semantic_id: super::semantic::AUTHORIZATION,
        previous_state: super::semantic::READY,
        new_state: super::semantic::FAILED,
        context: 9,
        timestamp: crate::system::state::uptime_ticks(),
        provenance: super::relation::RelationProvenance {
            origin: 202,
            source: super::relation::ProvenanceSource::Local,
            revision: 7,
        },
    };

    let matching_record = match super::attention::evaluate(&matching_delta) {
        Some(value) => value,
        None => return false,
    };

    let expected = ReflexTrigger {
        object_id: matching_record.object_id,
        semantic_id: matching_record.semantic_id,
        previous_state: matching_record.previous_state,
        new_state: matching_record.new_state,
        context: matching_record.context,
        timestamp: matching_record.timestamp,
        provenance: matching_record.provenance,
    };

    if evaluate(&matching_record) != Some(expected) {
        return false;
    }

    if evaluate(&matching_record) != Some(expected) {
        return false;
    }

    let non_matching_delta = super::delta::SemanticDelta {
        object_id: 78,
        semantic_id: super::semantic::AUTHORIZATION,
        previous_state: super::semantic::READY,
        new_state: super::semantic::ACTIVE,
        context: 10,
        timestamp: crate::system::state::uptime_ticks(),
        provenance: super::relation::RelationProvenance {
            origin: 203,
            source: super::relation::ProvenanceSource::Remote,
            revision: 8,
        },
    };

    let non_matching_record = match super::attention::evaluate(&non_matching_delta) {
        Some(value) => value,
        None => return false,
    };

    if evaluate(&non_matching_record).is_some() {
        return false;
    }

    if super::attention::count() != 0 {
        return false;
    }

    if super::delta::count() != delta_before {
        return false;
    }

    if observe().is_some() {
        return false;
    }

    super::attention::clear();
    true
}
