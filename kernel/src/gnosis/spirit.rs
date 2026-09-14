//! GNOSIS semantic spirit snapshot.
//!
//! REL-08 provides a bounded, read-only semantic coherence descriptor over
//! existing canonical GNOSIS and system state. It does not mutate canonical
//! registries, does not consume queues, and does not make decisions.

use super::graph;
use super::relation::{self, RelationTrust};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpiritSnapshot {
    pub active: bool,
    pub generation: u64,
    pub uptime_ticks: u64,
    pub soul_present: bool,
    pub attention_count: usize,
    pub degraded_count: usize,
    pub total_attention_score: u64,
    pub reflex_active: bool,
    pub knowledge_entries: usize,
    pub relation_count: usize,
    pub relation_registry_hash: u64,
    pub graph_valid: bool,
    pub graph_invalid_relations: usize,
    pub graph_duplicate_relations: usize,
    pub graph_disconnected_relations: usize,
    pub graph_node_count: usize,
    pub graph_context_count: usize,
    pub graph_semantic_count: usize,
}

pub const CAPACITY: usize = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RelationObservation {
    ids: [u64; relation::MAX_RELATIONS],
    trusts: [RelationTrust; relation::MAX_RELATIONS],
    integrities: [u64; relation::MAX_RELATIONS],
    count: usize,
    hash: u64,
}

#[inline]
fn capture_relation_observation() -> Option<RelationObservation> {
    let mut observation = RelationObservation {
        ids: [0; relation::MAX_RELATIONS],
        trusts: [RelationTrust::Invalid; relation::MAX_RELATIONS],
        integrities: [0; relation::MAX_RELATIONS],
        count: 0,
        hash: relation_registry_hash(),
    };
    let mut valid = true;

    relation::for_each(|id, _| {
        if !valid || observation.count >= relation::MAX_RELATIONS {
            valid = false;
            return;
        }

        let trust = match relation::trust(id) {
            Some(value) => value,
            None => {
                valid = false;
                return;
            }
        };

        let integrity = match relation::integrity(id) {
            Some(value) => value,
            None => {
                valid = false;
                return;
            }
        };

        observation.ids[observation.count] = id;
        observation.trusts[observation.count] = trust;
        observation.integrities[observation.count] = integrity;
        observation.count += 1;
    });

    if !valid {
        return None;
    }

    Some(observation)
}

#[inline]
fn relation_registry_hash() -> u64 {
    match relation::hash() {
        Some(value) => value,
        None => panic!("relation hash unavailable"),
    }
}

#[inline]
pub fn snapshot() -> SpiritSnapshot {
    let _soul = super::soul::observe();
    let homeostasis = super::homeostasis::observe();
    let reflex_active = super::reflex::observe().is_some();
    let graph_report = graph::validate();
    let graph_stats = graph::stats();

    SpiritSnapshot {
        active: super::context::is_active(),
        generation: super::context::generation(),
        uptime_ticks: super::context::uptime_ticks(),
        soul_present: true,
        attention_count: homeostasis.attention_count,
        degraded_count: homeostasis.degraded_count,
        total_attention_score: homeostasis.total_score,
        reflex_active,
        knowledge_entries: super::knowledge::entry_count(),
        relation_count: relation::count(),
        relation_registry_hash: relation_registry_hash(),
        graph_valid: graph_report.valid,
        graph_invalid_relations: graph_report.invalid_relations,
        graph_duplicate_relations: graph_report.duplicate_relations,
        graph_disconnected_relations: graph_report.disconnected_relation_count,
        graph_node_count: graph_stats.node_count,
        graph_context_count: graph_stats.context_count,
        graph_semantic_count: graph_stats.semantic_count,
    }
}

#[inline]
pub fn observe() -> SpiritSnapshot {
    snapshot()
}

#[inline]
pub fn self_test() -> bool {
    if CAPACITY != 1 {
        return false;
    }

    let delta_before = super::delta::count();
    let attention_before = super::attention::count();
    let knowledge_before = super::knowledge::entry_count();
    let relations_before = match capture_relation_observation() {
        Some(value) => value,
        None => return false,
    };
    let graph_stats_before = graph::stats();
    let graph_report_before = graph::validate();

    let first = snapshot();

    if !first.active || !first.soul_present {
        return false;
    }

    if first.attention_count != super::homeostasis::observe().attention_count
        || first.degraded_count != super::homeostasis::observe().degraded_count
        || first.total_attention_score != super::homeostasis::observe().total_score
    {
        return false;
    }

    if first.reflex_active != super::reflex::observe().is_some() {
        return false;
    }

    if first.knowledge_entries != super::knowledge::entry_count() {
        return false;
    }

    if first.relation_count != relation::count() {
        return false;
    }

    if first.relation_registry_hash != relation_registry_hash() {
        return false;
    }

    let graph_stats_now = graph::stats();
    let graph_report_now = graph::validate();

    if first.graph_valid != graph_report_now.valid
        || first.graph_invalid_relations != graph_report_now.invalid_relations
        || first.graph_duplicate_relations != graph_report_now.duplicate_relations
        || first.graph_disconnected_relations != graph_report_now.disconnected_relation_count
        || first.graph_node_count != graph_stats_now.node_count
        || first.graph_context_count != graph_stats_now.context_count
        || first.graph_semantic_count != graph_stats_now.semantic_count
    {
        return false;
    }

    if first.generation > super::context::generation() {
        return false;
    }

    if first.uptime_ticks > super::context::uptime_ticks() {
        return false;
    }

    let second = observe();

    if second.active != first.active
        || second.soul_present != first.soul_present
        || second.attention_count != first.attention_count
        || second.degraded_count != first.degraded_count
        || second.total_attention_score != first.total_attention_score
        || second.reflex_active != first.reflex_active
        || second.knowledge_entries != first.knowledge_entries
        || second.relation_count != first.relation_count
        || second.relation_registry_hash != first.relation_registry_hash
        || second.graph_valid != first.graph_valid
        || second.graph_invalid_relations != first.graph_invalid_relations
        || second.graph_duplicate_relations != first.graph_duplicate_relations
        || second.graph_disconnected_relations != first.graph_disconnected_relations
        || second.graph_node_count != first.graph_node_count
        || second.graph_context_count != first.graph_context_count
        || second.graph_semantic_count != first.graph_semantic_count
    {
        return false;
    }

    if super::delta::count() != delta_before
        || super::attention::count() != attention_before
        || super::knowledge::entry_count() != knowledge_before
    {
        return false;
    }

    let relations_after = match capture_relation_observation() {
        Some(value) => value,
        None => return false,
    };
    if relations_after != relations_before {
        return false;
    }

    let graph_stats_after = graph::stats();
    let graph_report_after = graph::validate();
    if graph_stats_after.relation_count != graph_stats_before.relation_count
        || graph_stats_after.node_count != graph_stats_before.node_count
        || graph_stats_after.source_node_count != graph_stats_before.source_node_count
        || graph_stats_after.sink_node_count != graph_stats_before.sink_node_count
        || graph_stats_after.isolated_node_count != graph_stats_before.isolated_node_count
        || graph_stats_after.self_relation_count != graph_stats_before.self_relation_count
        || graph_stats_after.context_count != graph_stats_before.context_count
        || graph_stats_after.semantic_count != graph_stats_before.semantic_count
    {
        return false;
    }

    if graph_report_after.valid != graph_report_before.valid
        || graph_report_after.checked_relations != graph_report_before.checked_relations
        || graph_report_after.invalid_relations != graph_report_before.invalid_relations
        || graph_report_after.duplicate_relations != graph_report_before.duplicate_relations
        || graph_report_after.disconnected_relation_count
            != graph_report_before.disconnected_relation_count
    {
        return false;
    }

    if core::mem::size_of::<SpiritSnapshot>() == 0 || core::mem::size_of::<SpiritSnapshot>() > 512 {
        return false;
    }

    if core::mem::size_of::<SpiritSnapshot>() > 1024 {
        return false;
    }

    true
}
