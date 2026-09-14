//! GNOSIS bounded Wisdom Engine.
//!
//! Wisdom is a derived constraint-aware judgment layer. It evaluates supplied
//! action candidates but never dispatches or executes them.
//!
//! Hard constraints are evaluated before utility. Utility therefore cannot
//! compensate for stale state, missing authority, insufficient trust or an
//! explicitly unsafe candidate.

use super::mesh::MeshNode;
use super::reason::InferenceRecord;
use super::semantic::SemanticId;
use super::trust::{self, AuthorizationDecision, AuthorizationRequest};

pub const SCALE: u32 = 10_000;
pub const MAX_CANDIDATES: usize = 16;
pub const MAX_WORK: usize = MAX_CANDIDATES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WisdomCandidate {
    pub action: SemanticId,
    pub goal_alignment_fixed: u32,
    pub confidence_fixed: u32,
    pub expected_outcome_fixed: u32,
    pub reversibility_fixed: u32,
    pub experience_fixed: u32,
    pub cost_fixed: u32,
    pub stale: bool,
    pub safe: bool,
    pub authorization: AuthorizationRequest,
    pub node: Option<MeshNode>,
}

impl WisdomCandidate {
    pub const fn empty() -> Self {
        Self {
            action: 0,
            goal_alignment_fixed: 0,
            confidence_fixed: 0,
            expected_outcome_fixed: 0,
            reversibility_fixed: 0,
            experience_fixed: 0,
            cost_fixed: SCALE,
            stale: true,
            safe: false,
            authorization: AuthorizationRequest {
                principal_id: 0,
                required_capability: 0,
                required_authority: 0,
                minimum_trust_fixed: SCALE,
            },
            node: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WisdomDenial {
    InvalidAction,
    Stale,
    Unsafe,
    Authority,
    Trust,
    Capability,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WisdomDisposition {
    Eligible,
    Denied(WisdomDenial),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WisdomEvaluation {
    pub action: SemanticId,
    pub disposition: WisdomDisposition,
    pub utility_fixed: u32,
    pub confidence_fixed: u32,
    pub cost_fixed: u32,
    pub source_inference: InferenceRecord,
}

impl WisdomEvaluation {
    pub const fn empty() -> Self {
        Self {
            action: 0,
            disposition: WisdomDisposition::Denied(WisdomDenial::InvalidAction),
            utility_fixed: 0,
            confidence_fixed: 0,
            cost_fixed: 0,
            source_inference: InferenceRecord::empty(),
        }
    }

    #[inline]
    pub const fn eligible(&self) -> bool {
        matches!(self.disposition, WisdomDisposition::Eligible)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WisdomReport {
    pub evaluated: usize,
    pub eligible: usize,
    pub denied: usize,
    pub selected_index: Option<usize>,
    pub work_used: usize,
    pub work_budget: usize,
    pub stopped_by_budget: bool,
    pub provisional: bool,
}

impl WisdomReport {
    pub const fn empty() -> Self {
        Self {
            evaluated: 0,
            eligible: 0,
            denied: 0,
            selected_index: None,
            work_used: 0,
            work_budget: MAX_WORK,
            stopped_by_budget: false,
            provisional: false,
        }
    }
}

#[inline]
fn utility(candidate: &WisdomCandidate) -> u32 {
    // The architecture defines a weighted utility over alignment, confidence,
    // expected outcome, reversibility and experience. This bounded first
    // implementation uses equal weights and subtracts cost after normalization.
    let sum = candidate
        .goal_alignment_fixed
        .min(SCALE)
        .saturating_add(candidate.confidence_fixed.min(SCALE))
        .saturating_add(candidate.expected_outcome_fixed.min(SCALE))
        .saturating_add(candidate.reversibility_fixed.min(SCALE))
        .saturating_add(candidate.experience_fixed.min(SCALE));

    let benefit = sum / 5;
    benefit.saturating_sub(candidate.cost_fixed.min(SCALE))
}

#[inline]
fn map_authorization(decision: AuthorizationDecision) -> Option<WisdomDenial> {
    match decision {
        AuthorizationDecision::Authorized => None,
        AuthorizationDecision::NoPrincipal => Some(WisdomDenial::Authority),
        AuthorizationDecision::MissingCapability => Some(WisdomDenial::Capability),
        AuthorizationDecision::MissingAuthority => Some(WisdomDenial::Authority),
        AuthorizationDecision::InsufficientTrust => Some(WisdomDenial::Trust),
    }
}

#[inline]
fn evaluate_candidate(candidate: WisdomCandidate, inference: InferenceRecord) -> WisdomEvaluation {
    if candidate.action == 0 || !super::semantic::is_valid(candidate.action) {
        return WisdomEvaluation {
            action: candidate.action,
            disposition: WisdomDisposition::Denied(WisdomDenial::InvalidAction),
            utility_fixed: 0,
            confidence_fixed: 0,
            cost_fixed: candidate.cost_fixed.min(SCALE),
            source_inference: inference,
        };
    }

    if candidate.stale {
        return WisdomEvaluation {
            action: candidate.action,
            disposition: WisdomDisposition::Denied(WisdomDenial::Stale),
            utility_fixed: 0,
            confidence_fixed: 0,
            cost_fixed: candidate.cost_fixed.min(SCALE),
            source_inference: inference,
        };
    }

    if !candidate.safe {
        return WisdomEvaluation {
            action: candidate.action,
            disposition: WisdomDisposition::Denied(WisdomDenial::Unsafe),
            utility_fixed: 0,
            confidence_fixed: 0,
            cost_fixed: candidate.cost_fixed.min(SCALE),
            source_inference: inference,
        };
    }

    // Hard authority/trust/capability constraints run before utility.
    if let Some(denial) =
        map_authorization(trust::authorize(candidate.authorization, candidate.node))
    {
        return WisdomEvaluation {
            action: candidate.action,
            disposition: WisdomDisposition::Denied(denial),
            utility_fixed: 0,
            confidence_fixed: 0,
            cost_fixed: candidate.cost_fixed.min(SCALE),
            source_inference: inference,
        };
    }

    let score = utility(&candidate);

    WisdomEvaluation {
        action: candidate.action,
        disposition: WisdomDisposition::Eligible,
        utility_fixed: score,
        confidence_fixed: candidate.confidence_fixed.min(SCALE),
        cost_fixed: candidate.cost_fixed.min(SCALE),
        source_inference: inference,
    }
}

/// Evaluate bounded candidates and select the highest-utility eligible result.
///
/// This function returns a proposed judgment only. It does not execute the
/// selected action and does not mutate canonical GNOSIS.
#[inline]
pub fn evaluate(
    candidates: &[WisdomCandidate],
    inference: InferenceRecord,
    output: &mut [WisdomEvaluation],
) -> WisdomReport {
    let mut report = WisdomReport::empty();
    let limit = candidates.len().min(MAX_CANDIDATES).min(output.len());

    if candidates.len() > limit {
        report.provisional = true;
    }

    let mut index = 0usize;
    let mut best_score = 0u32;
    let mut best_index = None;

    while index < limit {
        if report.work_used >= report.work_budget {
            report.stopped_by_budget = true;
            report.provisional = true;
            break;
        }

        report.work_used = report.work_used.saturating_add(1);
        report.evaluated = report.evaluated.saturating_add(1);

        let evaluation = evaluate_candidate(candidates[index], inference);
        output[index] = evaluation;

        if evaluation.eligible() {
            report.eligible = report.eligible.saturating_add(1);
            if best_index.is_none() || evaluation.utility_fixed > best_score {
                best_score = evaluation.utility_fixed;
                best_index = Some(index);
            }
        } else {
            report.denied = report.denied.saturating_add(1);
        }

        index += 1;
    }

    report.selected_index = best_index;
    report
}

#[inline]
pub fn self_test() -> bool {
    trust::clear();

    let provenance = super::relation::RelationProvenance {
        origin: 1,
        source: super::relation::ProvenanceSource::Local,
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

    let inference = InferenceRecord {
        object_id: 1,
        semantic_id: super::semantic::STATE_OBJECT,
        previous_state: super::semantic::READY,
        current_state: super::semantic::ACTIVE,
        predicted_state: super::semantic::ACTIVE,
        context: 1,
        kind: super::reason::InferenceKind::PredictionConsistent,
        truth: super::reason::TruthState::True,
        confidence_fixed: 9_000,
        uncertainty_fixed: 1_000,
        timestamp: 1,
        provenance,
    };

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

    let mut output = [WisdomEvaluation::empty(); MAX_CANDIDATES];
    let report = evaluate(&[candidate], inference, &mut output);
    let stale = evaluate_candidate(
        WisdomCandidate {
            stale: true,
            ..candidate
        },
        inference,
    );
    let unknown = evaluate_candidate(
        WisdomCandidate {
            authorization: AuthorizationRequest {
                principal_id: 0xFFFF,
                required_capability: 0,
                required_authority: 1,
                minimum_trust_fixed: 1,
            },
            ..candidate
        },
        inference,
    );

    trust::clear();

    report.eligible == 1
        && report.selected_index == Some(0)
        && output[0].eligible()
        && stale.disposition == WisdomDisposition::Denied(WisdomDenial::Stale)
        && unknown.disposition == WisdomDisposition::Denied(WisdomDenial::Authority)
}
