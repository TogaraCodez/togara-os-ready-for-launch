//! GNOSIS bounded proof/evidence validation.
//!
//! This release implements an inspectable deterministic proof boundary, not a
//! cryptographic proof system. It validates the evidence and hard constraints
//! attached to a derived decision before that decision can become Proposed.
//! No action is executed and no canonical GNOSIS state is mutated here.

//use super::decision::DecisionConstraints;
use super::reason::{InferenceRecord, TruthState};
use super::semantic;
use super::wisdom::WisdomEvaluation;
use crate::gnosis::decision::DecisionConstraints;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProofStatus {
    NotEvaluated,
    Valid,
    InvalidEvidence,
    InvalidConstraints,
    InvalidAction,
    Stale,
    MissingAuthorityCheck,
    MissingTrustCheck,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProofRecord {
    pub status: ProofStatus,
    pub goal: semantic::SemanticId,
    pub action: semantic::SemanticId,
    pub evidence_object: semantic::ObjectId,
    pub evidence_context: semantic::ContextId,
    pub source_revision: u64,
    pub truth: TruthState,
    pub confidence_fixed: u32,
    pub uncertainty_fixed: u32,
    pub constraints_hash: u32,
}

impl ProofRecord {
    pub const fn empty() -> Self {
        Self {
            status: ProofStatus::NotEvaluated,
            goal: 0,
            action: 0,
            evidence_object: 0,
            evidence_context: 0,
            source_revision: 0,
            truth: TruthState::Unknown,
            confidence_fixed: 0,
            uncertainty_fixed: 10_000,
            constraints_hash: 0,
        }
    }

    #[inline]
    pub const fn valid(&self) -> bool {
        matches!(self.status, ProofStatus::Valid)
    }
}

#[inline]
fn constraints_hash(constraints: DecisionConstraints) -> u32 {
    let mut value = 0x811C_9DC5u32;
    value ^= constraints.safe as u32;
    value = value.wrapping_mul(16_777_619);
    value ^= constraints.fresh as u32;
    value = value.wrapping_mul(16_777_619);
    value ^= constraints.authority_checked as u32;
    value = value.wrapping_mul(16_777_619);
    value ^= constraints.trust_checked as u32;
    value = value.wrapping_mul(16_777_619);
    value ^= constraints.execution_allowed as u32;
    value.wrapping_mul(16_777_619)
}

#[inline]
fn valid_inference(inference: &InferenceRecord) -> bool {
    inference.object_id != 0
        && semantic::is_valid(inference.semantic_id)
        && inference.provenance.revision != 0
        && inference.timestamp != 0
        && inference.confidence_fixed <= 10_000
        && inference.uncertainty_fixed <= 10_000
}

/// Validate the evidence and hard constraints for one Wisdom-approved
/// evaluation. The result is evidence only; it grants no execution authority.
#[inline]
pub fn build(
    goal: semantic::SemanticId,
    evaluation: &WisdomEvaluation,
    constraints: DecisionConstraints,
) -> ProofRecord {
    let inference = &evaluation.source_inference;
    let mut proof = ProofRecord {
        status: ProofStatus::InvalidEvidence,
        goal,
        action: evaluation.action,
        evidence_object: inference.object_id,
        evidence_context: inference.context,
        source_revision: inference.provenance.revision,
        truth: inference.truth,
        confidence_fixed: inference.confidence_fixed.min(10_000),
        uncertainty_fixed: inference.uncertainty_fixed.min(10_000),
        constraints_hash: constraints_hash(constraints),
    };

    if goal == 0 || !semantic::is_valid(goal) || evaluation.action == 0 {
        proof.status = ProofStatus::InvalidAction;
        return proof;
    }

    if !semantic::is_valid(evaluation.action) {
        proof.status = ProofStatus::InvalidAction;
        return proof;
    }

    if !valid_inference(inference) {
        proof.status = ProofStatus::InvalidEvidence;
        return proof;
    }

    if matches!(inference.truth, TruthState::Stale) {
        proof.status = ProofStatus::Stale;
        return proof;
    }

    if matches!(
        inference.truth,
        TruthState::Unknown | TruthState::Conflicting
    ) {
        proof.status = ProofStatus::InvalidEvidence;
        return proof;
    }

    if !constraints.safe || !constraints.fresh {
        proof.status = ProofStatus::InvalidConstraints;
        return proof;
    }

    if !constraints.authority_checked {
        proof.status = ProofStatus::MissingAuthorityCheck;
        return proof;
    }

    if !constraints.trust_checked {
        proof.status = ProofStatus::MissingTrustCheck;
        return proof;
    }

    // The first cognitive release is deliberately non-executing. A true
    // execution permission would require a later, separately verified gate.
    if constraints.execution_allowed {
        proof.status = ProofStatus::InvalidConstraints;
        return proof;
    }

    proof.status = ProofStatus::Valid;
    proof
}

#[inline]
pub fn self_test() -> bool {
    let provenance = super::relation::RelationProvenance {
        origin: 1,
        source: super::relation::ProvenanceSource::Local,
        revision: 1,
    };

    let inference = InferenceRecord {
        object_id: 1,
        semantic_id: semantic::STATE_OBJECT,
        previous_state: semantic::READY,
        current_state: semantic::ACTIVE,
        predicted_state: semantic::ACTIVE,
        context: 1,
        kind: super::reason::InferenceKind::PredictionConsistent,
        truth: TruthState::True,
        confidence_fixed: 9_000,
        uncertainty_fixed: 1_000,
        timestamp: 1,
        provenance,
    };

    let evaluation = WisdomEvaluation {
        action: semantic::VERIFY,
        disposition: super::wisdom::WisdomDisposition::Eligible,
        utility_fixed: 8_000,
        confidence_fixed: 9_000,
        cost_fixed: 1_000,
        source_inference: inference,
    };

    let valid = build(
        semantic::STATE_OBJECT,
        &evaluation,
        DecisionConstraints::cognitive_release(),
    );

    let execution_denied = build(
        semantic::STATE_OBJECT,
        &evaluation,
        DecisionConstraints {
            execution_allowed: true,
            ..DecisionConstraints::cognitive_release()
        },
    );

    valid.valid() && execution_denied.status == ProofStatus::InvalidConstraints
}
