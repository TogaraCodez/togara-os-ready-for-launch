//! GNOSIS bounded Self Model.
//!
//! Represents the operational self as a derived capability/resource snapshot.
//! It never rewrites Body, Soul, Spirit, or canonical KOBJ state.

use super::body::BodySnapshot;
use super::relations::{ProvenanceSource, RelationProvenance};

pub const SCALE: u32 = 10_000;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelfModel {
    pub generation: u64,
    pub uptime_ticks: u64,
    pub keyboard_events: u64,
    pub total_memory_bytes: u64,
    pub usable_memory_bytes: u64,
    pub allocated_memory_bytes: u64,
    pub reserved_memory_bytes: u64,
    pub reclaimed_memory_bytes: u64,
    pub compute_capacity_fixed: u32,
    pub operational_confidence_fixed: u32,
    pub provenance: RelationProvenance,
}
impl SelfModel {
    pub const fn empty() -> Self {
        Self {
            generation: 0,
            uptime_ticks: 0,
            keyboard_events: 0,
            total_memory_bytes: 0,
            usable_memory_bytes: 0,
            allocated_memory_bytes: 0,
            reserved_memory_bytes: 0,
            reclaimed_memory_bytes: 0,
            compute_capacity_fixed: 0,
            operational_confidence_fixed: 0,
            provenance: RelationProvenance {
                origin: 0,
                source: ProvenanceSource::Local,
                revision: 0,
            },
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelfAssessment {
    pub healthy: bool,
    pub capacity_fixed: u32,
    pub confidence_fixed: u32,
    pub reason_code: u8,
}
fn derive(body: BodySnapshot, provenance: RelationProvenance) -> SelfModel {
    let capacity = if body.total_memory_bytes == 0 {
        0
    } else {
        ((body.usable_memory_bytes.saturating_mul(SCALE as u64)) / body.total_memory_bytes)
            .min(SCALE as u64) as u32
    };
    let conf = if body.timer_frequency == 0 { 0 } else { SCALE };
    SelfModel {
        generation: body.keyboard_events,
        uptime_ticks: body.uptime_ticks,
        keyboard_events: body.keyboard_events,
        total_memory_bytes: body.total_memory_bytes,
        usable_memory_bytes: body.usable_memory_bytes,
        allocated_memory_bytes: body.allocated_memory_bytes,
        reserved_memory_bytes: body.reserved_memory_bytes,
        reclaimed_memory_bytes: body.reclaimed_memory_bytes,
        compute_capacity_fixed: capacity,
        operational_confidence_fixed: conf,
        provenance,
    }
}
pub fn snapshot(provenance: RelationProvenance) -> SelfModel {
    derive(super::body::snapshot(), provenance)
}
pub fn assess(model: SelfModel) -> SelfAssessment {
    let healthy = model.operational_confidence_fixed > 0
        && model.usable_memory_bytes <= model.total_memory_bytes;
    SelfAssessment {
        healthy,
        capacity_fixed: model.compute_capacity_fixed,
        confidence_fixed: model.operational_confidence_fixed,
        reason_code: if healthy { 1 } else { 2 },
    }
}
pub fn self_test() -> bool {
    let p = RelationProvenance {
        origin: 1,
        source: ProvenanceSource::Local,
        revision: 1,
    };
    let s = snapshot(p);
    let a = assess(s);
    a.confidence_fixed > 0 && a.capacity_fixed <= SCALE
}
