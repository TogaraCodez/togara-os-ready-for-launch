//! GNOSIS Decision — proof-carrying, non-executing decision artifact.
//!
//! Decision is a derived audit boundary. It cannot execute, mutate canonical
//! state, or bypass Wisdom. Unknown/insufficient authority is DENY.

use super::relation::{ProvenanceSource, RelationProvenance};
use super::semantic::{ContextId, ObjectId, SemanticId};
use super::wisdom::{WisdomCandidate, WisdomEvaluation, WisdomReport};

pub const SCALE: u32 = 10_000;
pub const MAX_CANDIDATES: usize = 16;
pub const MAX_WORK: usize = MAX_CANDIDATES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecisionStatus {
    Proposed,
    Denied,
    Invalid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanonicalValidation {
    Valid,
    Stale,
    Invalid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorityOutcome {
    Allow,
    Deny,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct DecisionRequest {
    pub candidates: [WisdomCandidate; MAX_CANDIDATES],
    pub candidate_count: usize,
    pub canonical_validation: CanonicalValidation,
    pub evidence_reference: u64,
    pub rationale_code: u16,
    pub work_budget: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct DecisionArtifact {
    pub status: DecisionStatus,
    pub object_id: ObjectId,
    pub semantic_id: SemanticId,
    pub context: ContextId,
    pub evidence_reference: u64,
    pub authority: AuthorityOutcome,
    pub canonical_validation: CanonicalValidation,
    pub confidence_fixed: u32,
    pub rationale_code: u16,
    pub work_used: usize,
    pub provenance: RelationProvenance,
}

impl DecisionArtifact {
    pub const fn denied() -> Self {
        Self {
            status: DecisionStatus::Invalid,
            object_id: 0,
            semantic_id: 0,
            context: 0,
            evidence_reference: 0,
            authority: AuthorityOutcome::Deny,
            canonical_validation: CanonicalValidation::Invalid,
            confidence_fixed: 0,
            rationale_code: 0,
            work_used: 0,
            provenance: RelationProvenance {
                origin: 0,
                source: ProvenanceSource::Local,
                revision: 0,
            },
        }
    }
}

/// Fixed-capacity decision constraints marker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecisionConstraints {
    pub safe: bool,
    pub fresh: bool,
    pub authority_checked: bool,
    pub trust_checked: bool,
    pub execution_allowed: bool,
}

impl DecisionConstraints {
    pub const fn cognitive_release() -> Self {
        Self {
            safe: true,
            fresh: true,
            authority_checked: true,
            trust_checked: true,
            execution_allowed: false, // explicitly no execution at this stage
        }
    }
}

#[inline]
fn provenance_valid(p: &RelationProvenance) -> bool {
    p.revision != 0
        && matches!(
            p.source,
            ProvenanceSource::Local | ProvenanceSource::Remote | ProvenanceSource::Recovered
        )
}

/// Evaluate candidates through Wisdom and produce a proposed decision.
///
/// This does not execute any action and does not mutate canonical GNOSIS.
pub fn decide(
    request: DecisionRequest,
) -> (
    DecisionArtifact,
    WisdomReport,
    [WisdomEvaluation; MAX_CANDIDATES],
) {
    let mut result = DecisionArtifact::denied();
    let mut evaluations = [WisdomEvaluation::empty(); MAX_CANDIDATES];

    result.evidence_reference = request.evidence_reference;
    result.canonical_validation = request.canonical_validation;
    result.rationale_code = request.rationale_code;
    result.work_used = 1;

    let budget = request.work_budget.min(MAX_WORK);
    if budget == 0 {
        return (result, WisdomReport::empty(), evaluations);
    }

    // Canonical freshness is a hard gate.
    if request.canonical_validation != CanonicalValidation::Valid {
        result.status = DecisionStatus::Denied;
        result.authority = AuthorityOutcome::Deny;
        return (result, WisdomReport::empty(), evaluations);
    }

    // Run Wisdom evaluation over supplied candidates.
    let report = super::wisdom::evaluate(
        &request.candidates[..request.candidate_count.min(MAX_CANDIDATES)],
        super::reason::InferenceRecord::empty(),
        &mut evaluations,
    );

    result.work_used = result.work_used.saturating_add(report.work_used);

    // If no eligible candidate, deny.
    if report.eligible == 0 || report.selected_index.is_none() {
        result.status = DecisionStatus::Denied;
        result.authority = AuthorityOutcome::Deny;
        return (result, report, evaluations);
    }

    // For now, we treat any eligible Wisdom result as authority-allowed.
    let idx = report.selected_index.unwrap();
    let chosen = &evaluations[idx];

    result.status = DecisionStatus::Proposed;
    result.authority = AuthorityOutcome::Allow;
    result.object_id = 1; // placeholder; wire to real object as needed
    result.semantic_id = chosen.action;
    result.context = 0;
    result.confidence_fixed = chosen.confidence_fixed.min(SCALE);
    result.provenance = RelationProvenance {
        origin: 1,
        source: ProvenanceSource::Local,
        revision: 1,
    };

    (result, report, evaluations)
}

pub fn self_test() -> bool {
    use super::relation::ProvenanceSource;
    use super::relation::RelationProvenance;
    use super::trust::{self, AuthorizationRequest};

    trust::clear();

    let provenance = RelationProvenance {
        origin: 1,
        source: ProvenanceSource::Local,
        revision: 1,
    };

    if !trust::upsert(trust::TrustPrincipal {
        principal_id: 7,
        trust_fixed: 9_000,
        authority_mask: 1,
        revision: 1,
        provenance,
    }) {
        return false;
    }

    let candidate = WisdomCandidate {
        action: super::semantic::VERIFY,
        goal_alignment_fixed: 9_000,
        confidence_fixed: 9_000,
        expected_outcome_fixed: 9_000,
        reversibility_fixed: 9_000,
        experience_fixed: 9_000,
        cost_fixed: 1_000,
        stale: false,
        safe: true,
        authorization: AuthorizationRequest {
            principal_id: 7,
            required_capability: 0,
            required_authority: 1,
            minimum_trust_fixed: 8_000,
        },
        node: None,
    };

    let mut candidates = [WisdomCandidate::empty(); MAX_CANDIDATES];
    candidates[0] = candidate;

    let request = DecisionRequest {
        candidates,
        candidate_count: 1,
        canonical_validation: CanonicalValidation::Valid,
        evidence_reference: 42,
        rationale_code: 1,
        work_budget: 8,
    };

    let (artifact, report, evaluations) = decide(request);

    let stale_candidate = WisdomCandidate {
        stale: true,
        ..candidate
    };
    let mut stale_candidates = [WisdomCandidate::empty(); MAX_CANDIDATES];
    stale_candidates[0] = stale_candidate;

    let stale_request = DecisionRequest {
        candidates: stale_candidates,
        candidate_count: 1,
        canonical_validation: CanonicalValidation::Valid,
        evidence_reference: 42,
        rationale_code: 1,
        work_budget: 8,
    };

    let (stale_artifact, stale_report, _) = decide(stale_request);

    trust::clear();

    artifact.status == DecisionStatus::Proposed
        && artifact.authority == AuthorityOutcome::Allow
        && report.eligible == 1
        && report.selected_index == Some(0)
        && evaluations[0].eligible()
        && stale_artifact.status == DecisionStatus::Denied
        && stale_report.eligible == 0
}
