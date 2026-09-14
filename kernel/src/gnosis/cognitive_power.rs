//! GNOSIS Cognitive Power Management.
//!
//! Selects the minimum sufficient compute route subject to quality, safety,
//! authority, latency, and resource constraints.

use super::compute_fabric::{self, ComputeLocation, ComputeRequest};
use super::local_compute::LocalComputeResult;
use super::relations::{ProvenanceSource, RelationProvenance};
use super::self_model::SelfModel;

pub const SCALE: u32 = 10_000;
pub const MAX_WORK: usize = 512;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PowerRequest {
    pub compute: ComputeRequest,
    pub required_quality_fixed: u32,
    pub max_cost_fixed: u32,
    pub max_latency_ticks: u64,
    pub prefer_local: bool,
    pub provenance: RelationProvenance,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PowerDecision {
    pub location: ComputeLocation,
    pub admitted: bool,
    pub quality_fixed: u32,
    pub estimated_work: u32,
    pub cost_fixed: u32,
    pub work_used: usize,
    pub reason_code: u8,
    pub provenance: RelationProvenance,
}
impl PowerDecision {
    pub const fn rejected() -> Self {
        Self {
            location: ComputeLocation::Defer,
            admitted: false,
            quality_fixed: 0,
            estimated_work: 0,
            cost_fixed: 0,
            work_used: 0,
            reason_code: 0,
            provenance: RelationProvenance {
                origin: 0,
                source: ProvenanceSource::Local,
                revision: 0,
            },
        }
    }
}
fn valid_p(p: &RelationProvenance) -> bool {
    p.revision != 0
        && matches!(
            p.source,
            ProvenanceSource::Local | ProvenanceSource::Remote | ProvenanceSource::Recovered
        )
}
pub fn arbitrate(
    request: PowerRequest,
    local: Option<LocalComputeResult>,
    self_model: SelfModel,
) -> PowerDecision {
    let mut d = PowerDecision::rejected();
    d.provenance = request.provenance;
    d.estimated_work = request.compute.estimated_work;
    if !valid_p(&request.provenance) || self_model.operational_confidence_fixed == 0 {
        return d;
    }
    if request.compute.deadline_ticks != 0
        && request.max_latency_ticks != 0
        && request.compute.deadline_ticks > request.max_latency_ticks
    {
        return d;
    }
    let c = compute_fabric::decide(request.compute, local);
    d.location = c.location;
    d.admitted = c.admitted;
    d.quality_fixed = c.utility_fixed.min(SCALE);
    d.cost_fixed = c.estimated_work.min(u32::MAX);
    d.work_used = c.estimated_work as usize;
    d.reason_code = if c.admitted
        && d.quality_fixed >= request.required_quality_fixed
        && d.cost_fixed <= request.max_cost_fixed
    {
        1
    } else {
        2
    };
    if d.reason_code != 1 {
        d.admitted = false;
        d.location = ComputeLocation::Defer
    }
    d
}
pub fn self_test() -> bool {
    let p = RelationProvenance {
        origin: 1,
        source: ProvenanceSource::Local,
        revision: 1,
    };
    let req = ComputeRequest {
        semantic_id: super::semantic::EXECUTE,
        context: 1,
        estimated_work: 4,
        max_work: 16,
        deadline_ticks: 10,
        required_capability: 0,
        minimum_trust_fixed: 0,
        provenance: p,
    };
    let local = LocalComputeResult {
        disposition: super::local_compute::ComputeDisposition::Evaluated,
        object_id: 1,
        semantic_id: super::semantic::EXECUTE,
        baseline_state: 1,
        hypothetical_state: 2,
        delta: 1,
        confidence_fixed: 9000,
        utility_fixed: 9000,
        work_used: 4,
    };
    let s = super::self_model::snapshot(p);
    let d = arbitrate(
        PowerRequest {
            compute: req,
            required_quality_fixed: 5000,
            max_cost_fixed: 16,
            max_latency_ticks: 20,
            prefer_local: true,
            provenance: p,
        },
        Some(local),
        s,
    );
    d.admitted && d.location == ComputeLocation::Local
}
