//! GNOSIS REL-14 — bounded compute arbitration.
//!
//! The first fabric implementation is intentionally conservative: it ranks
//! LOCAL execution versus DEFER. Remote execution is not invented here.
//! Hard constraints always dominate utility.

use super::local_compute::{self, ComputeDisposition, LocalComputeResult};
use super::relation::{ProvenanceSource, RelationProvenance};
use super::semantic::{ContextId, SemanticId};

pub const SCALE: u32 = 10_000;
pub const MAX_CANDIDATES: usize = 8;
pub const MAX_WORK: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComputeLocation {
    Local,
    Defer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComputeRequest {
    pub semantic_id: SemanticId,
    pub context: ContextId,
    pub estimated_work: u32,
    pub max_work: u32,
    pub deadline_ticks: u64,
    pub required_capability: u32,
    pub minimum_trust_fixed: u32,
    pub provenance: RelationProvenance,
}

impl ComputeRequest {
    pub const fn empty() -> Self {
        Self {
            semantic_id: 0,
            context: 0,
            estimated_work: 0,
            max_work: 0,
            deadline_ticks: 0,
            required_capability: 0,
            minimum_trust_fixed: 0,
            provenance: RelationProvenance {
                origin: 0,
                source: ProvenanceSource::Local,
                revision: 0,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComputeDecision {
    pub location: ComputeLocation,
    pub admitted: bool,
    pub utility_fixed: u32,
    pub estimated_work: u32,
    pub reason: u8,
}

impl ComputeDecision {
    pub const fn rejected() -> Self {
        Self {
            location: ComputeLocation::Defer,
            admitted: false,
            utility_fixed: 0,
            estimated_work: 0,
            reason: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComputeReport {
    pub considered: usize,
    pub admitted: usize,
    pub deferred: usize,
    pub work_used: usize,
    pub work_budget: usize,
    pub stopped_by_budget: bool,
    pub latest: ComputeDecision,
}

impl ComputeReport {
    pub const fn empty() -> Self {
        Self {
            considered: 0,
            admitted: 0,
            deferred: 0,
            work_used: 0,
            work_budget: MAX_WORK,
            stopped_by_budget: false,
            latest: ComputeDecision::rejected(),
        }
    }
}

#[inline]
pub fn decide(request: ComputeRequest, local: Option<LocalComputeResult>) -> ComputeDecision {
    if request.semantic_id == 0
        || request.provenance.revision == 0
        || request.estimated_work == 0
        || request.max_work == 0
        || request.estimated_work > request.max_work
    {
        return ComputeDecision::rejected();
    }

    let utility = match local {
        Some(result) if result.disposition == ComputeDisposition::Evaluated => {
            result.utility_fixed.min(SCALE)
        }
        _ => 0,
    };

    if !matches!(local, Some(result) if result.disposition == ComputeDisposition::Evaluated) {
        return ComputeDecision {
            location: ComputeLocation::Defer,
            admitted: false,
            utility_fixed: 0,
            estimated_work: request.estimated_work,
            reason: 2,
        };
    }

    ComputeDecision {
        location: ComputeLocation::Local,
        admitted: true,
        utility_fixed: utility,
        estimated_work: request.estimated_work,
        reason: 1,
    }
}

#[inline]
pub fn arbitrate(request: ComputeRequest, local: Option<LocalComputeResult>) -> ComputeDecision {
    decide(request, local)
}

#[inline]
pub fn evaluate_speculation(
    request: ComputeRequest,
    scenario: super::speculation::SpeculativeScenario,
) -> ComputeDecision {
    let result = local_compute::evaluate(scenario);
    decide(request, Some(result))
}

#[inline]
pub fn self_test() -> bool {
    let request = ComputeRequest {
        semantic_id: 2,
        context: 3,
        estimated_work: 4,
        max_work: 16,
        deadline_ticks: 100,
        required_capability: 0,
        minimum_trust_fixed: 0,
        provenance: RelationProvenance {
            origin: 1,
            source: ProvenanceSource::Local,
            revision: 1,
        },
    };

    let scenario = super::speculation::SpeculativeScenario {
        kind: super::speculation::SpeculationKind::PredictedAlternative,
        object_id: 1,
        semantic_id: 2,
        context: 3,
        baseline_state: 10,
        hypothetical_state: 20,
        previous_state: 9,
        support: 3,
        confidence_fixed: 8_000,
        uncertainty_fixed: 2_000,
        horizon: 1,
        timestamp: 100,
        provenance: request.provenance,
    };

    let decision = evaluate_speculation(request, scenario);
    if decision.location != ComputeLocation::Local || !decision.admitted {
        return false;
    }

    let rejected = LocalComputeResult::empty();
    let rejected_decision = decide(request, Some(rejected));
    if rejected_decision.location != ComputeLocation::Defer || rejected_decision.admitted {
        return false;
    }

    let invalid_request = ComputeRequest {
        estimated_work: 17,
        max_work: 16,
        ..request
    };
    if decide(invalid_request, Some(local_compute::evaluate(scenario))).admitted {
        return false;
    }

    true
}
