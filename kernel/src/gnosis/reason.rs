//! GNOSIS bounded semantic reasoner.
//!
//! Reason is a derived deliberative layer. It observes the Conscious Workspace
//! and produces bounded inferences without mutating canonical GNOSIS.
//!
//! The implementation deliberately starts with deterministic symbolic rules.
//! FAST/NORMAL/DEEP are explicit work budgets rather than claims of a neural
//! reasoning engine.

use super::conscious::{self, WorkspaceItem};
use super::semantic::{ContextId, ObjectId, SemanticId};

pub const SCALE: u32 = 10_000;
pub const MAX_INFERENCES: usize = conscious::MAX_WORKSPACE;
pub const MAX_WORK: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReasonMode {
    Fast,
    Normal,
    Deep,
}

impl ReasonMode {
    #[inline]
    pub const fn work_budget(self) -> usize {
        match self {
            Self::Fast => 8,
            Self::Normal => 32,
            Self::Deep => MAX_WORK,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TruthState {
    True,
    False,
    Unknown,
    Conflicting,
    Stale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InferenceKind {
    Transition,
    PredictionConsistent,
    PredictionMismatch,
    Observation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InferenceRecord {
    pub object_id: ObjectId,
    pub semantic_id: SemanticId,
    pub previous_state: SemanticId,
    pub current_state: SemanticId,
    pub predicted_state: SemanticId,
    pub context: ContextId,
    pub kind: InferenceKind,
    pub truth: TruthState,
    pub confidence_fixed: u32,
    pub uncertainty_fixed: u32,
    pub timestamp: u64,
    pub provenance: super::relation::RelationProvenance,
}

impl InferenceRecord {
    pub const fn empty() -> Self {
        Self {
            object_id: 0,
            semantic_id: 0,
            previous_state: 0,
            current_state: 0,
            predicted_state: 0,
            context: 0,
            kind: InferenceKind::Observation,
            truth: TruthState::Unknown,
            confidence_fixed: 0,
            uncertainty_fixed: SCALE,
            timestamp: 0,
            provenance: super::relation::RelationProvenance {
                origin: 0,
                source: super::relation::ProvenanceSource::Local,
                revision: 0,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReasonReport {
    pub observed: usize,
    pub inferred: usize,
    pub unknown: usize,
    pub conflicting: usize,
    pub stale: usize,
    pub work_used: usize,
    pub work_budget: usize,
    pub stopped_by_budget: bool,
    pub provisional: bool,
}

impl ReasonReport {
    pub const fn empty(mode: ReasonMode) -> Self {
        Self {
            observed: 0,
            inferred: 0,
            unknown: 0,
            conflicting: 0,
            stale: 0,
            work_used: 0,
            work_budget: mode.work_budget(),
            stopped_by_budget: false,
            provisional: false,
        }
    }
}

#[inline]
fn confidence(item: &WorkspaceItem) -> u32 {
    let salience = item.salience_fixed.min(SCALE);
    let certainty = SCALE.saturating_sub(item.uncertainty_fixed.min(SCALE));
    salience.saturating_mul(certainty).saturating_div(SCALE)
}

#[inline]
fn infer_item(item: WorkspaceItem) -> InferenceRecord {
    let certainty = confidence(&item);
    let mut kind = InferenceKind::Observation;
    let mut truth = TruthState::Unknown;

    if item.provenance.revision == 0 || item.object_id == 0 || item.semantic_id == 0 {
        return InferenceRecord {
            object_id: item.object_id,
            semantic_id: item.semantic_id,
            previous_state: item.previous_state,
            current_state: item.current_state,
            predicted_state: item.predicted_state,
            context: item.context,
            kind,
            truth,
            confidence_fixed: 0,
            uncertainty_fixed: SCALE,
            timestamp: item.timestamp,
            provenance: item.provenance,
        };
    }

    if item.previous_state != 0 && item.previous_state != item.current_state {
        kind = InferenceKind::Transition;
        truth = TruthState::True;
    }

    if item.predicted_state != 0 {
        if item.predicted_state == item.current_state {
            kind = InferenceKind::PredictionConsistent;
            truth = TruthState::True;
        } else if item.previous_state != 0 && item.previous_state == item.current_state {
            kind = InferenceKind::PredictionMismatch;
            truth = TruthState::False;
        } else {
            kind = InferenceKind::PredictionMismatch;
            truth = TruthState::Conflicting;
        }
    }

    InferenceRecord {
        object_id: item.object_id,
        semantic_id: item.semantic_id,
        previous_state: item.previous_state,
        current_state: item.current_state,
        predicted_state: item.predicted_state,
        context: item.context,
        kind,
        truth,
        confidence_fixed: certainty,
        uncertainty_fixed: item.uncertainty_fixed.min(SCALE),
        timestamp: item.timestamp,
        provenance: item.provenance,
    }
}

#[inline]
fn process_item(
    item: WorkspaceItem,
    output: &mut [InferenceRecord],
    output_used: &mut usize,
    report: &mut ReasonReport,
) {
    if *output_used >= output.len() {
        report.provisional = true;
        return;
    }

    let inference = infer_item(item);
    output[*output_used] = inference;
    *output_used += 1;
    report.inferred = report.inferred.saturating_add(1);

    match inference.truth {
        TruthState::Unknown => report.unknown = report.unknown.saturating_add(1),
        TruthState::Conflicting => report.conflicting = report.conflicting.saturating_add(1),
        TruthState::Stale => report.stale = report.stale.saturating_add(1),
        TruthState::True | TruthState::False => {}
    }
}

/// Reason over the current Conscious Workspace.
///
/// The workspace is observed through its read-only iteration boundary. No
/// canonical object, Delta, Attention record or Subconscious state is mutated.
#[inline]
pub fn process(mode: ReasonMode, output: &mut [InferenceRecord]) -> ReasonReport {
    let mut report = ReasonReport::empty(mode);
    let capacity = output.len().min(MAX_INFERENCES);

    if capacity == 0 {
        report.provisional = true;
        return report;
    }

    let mut used = 0usize;
    conscious::for_each(|item| {
        if used >= capacity || report.work_used >= report.work_budget {
            report.stopped_by_budget = true;
            report.provisional = true;
            return;
        }

        report.observed = report.observed.saturating_add(1);
        report.work_used = report.work_used.saturating_add(1);
        process_item(*item, &mut output[..capacity], &mut used, &mut report);
    });

    report
}

/// Reason over a caller-owned bounded scene. This is useful for deterministic
/// tests and for future callers that already possess a detached workspace.
#[inline]
pub fn evaluate(
    items: &[WorkspaceItem],
    mode: ReasonMode,
    output: &mut [InferenceRecord],
) -> ReasonReport {
    let mut report = ReasonReport::empty(mode);
    let limit = items.len().min(MAX_INFERENCES).min(output.len());
    let mut index = 0usize;

    while index < limit {
        if report.work_used >= report.work_budget {
            report.stopped_by_budget = true;
            report.provisional = true;
            break;
        }

        report.observed = report.observed.saturating_add(1);
        report.work_used = report.work_used.saturating_add(1);
        process_item(items[index], output, &mut index, &mut report);
        index = report.inferred;
    }

    if items.len() > limit {
        report.provisional = true;
    }

    report
}

#[inline]
pub fn self_test() -> bool {
    let provenance = super::relation::RelationProvenance {
        origin: 1,
        source: super::relation::ProvenanceSource::Local,
        revision: 1,
    };

    let items = [WorkspaceItem {
        object_id: 1,
        semantic_id: super::semantic::STATE_OBJECT,
        previous_state: super::semantic::READY,
        current_state: super::semantic::ACTIVE,
        predicted_state: super::semantic::ACTIVE,
        context: 7,
        salience_fixed: 8_000,
        uncertainty_fixed: 1_000,
        support: 3,
        timestamp: 10,
        provenance,
        source: super::conscious::WorkspaceSource::Subconscious,
    }];

    let mut output = [InferenceRecord::empty(); MAX_INFERENCES];
    let report = evaluate(&items, ReasonMode::Fast, &mut output);

    report.inferred == 1
        && report.unknown == 0
        && output[0].truth == TruthState::True
        && output[0].kind == InferenceKind::PredictionConsistent
        && output[0].confidence_fixed > 0
}
