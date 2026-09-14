//! GNOSIS REL-12 — bounded local speculative computation.
//!
//! Evaluates an existing speculative branch without mutating canonical state.
//! This is deliberately a deterministic, fixed-cost evaluator, not an
//! arbitrary code executor or planner.

use super::conscious::WorkspaceItem;
use super::speculation::SpeculativeScenario;

pub const SCALE: u32 = 10_000;
pub const MAX_WORK: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComputeDisposition {
    Evaluated,
    Rejected,
    BudgetExhausted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalComputeResult {
    pub disposition: ComputeDisposition,
    pub object_id: u64,
    pub semantic_id: u32,
    pub baseline_state: u32,
    pub hypothetical_state: u32,
    pub delta: u32,
    pub confidence_fixed: u32,
    pub utility_fixed: u32,
    pub work_used: usize,
}

impl LocalComputeResult {
    pub const fn empty() -> Self {
        Self {
            disposition: ComputeDisposition::Rejected,
            object_id: 0,
            semantic_id: 0,
            baseline_state: 0,
            hypothetical_state: 0,
            delta: 0,
            confidence_fixed: 0,
            utility_fixed: 0,
            work_used: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalComputeReport {
    pub evaluated: usize,
    pub rejected: usize,
    pub work_used: usize,
    pub work_budget: usize,
    pub stopped_by_budget: bool,
    pub latest: LocalComputeResult,
}

impl LocalComputeReport {
    pub const fn empty() -> Self {
        Self {
            evaluated: 0,
            rejected: 0,
            work_used: 0,
            work_budget: MAX_WORK,
            stopped_by_budget: false,
            latest: LocalComputeResult::empty(),
        }
    }
}

/// Evaluate one speculative branch.
///
/// The evaluator computes the absolute state distance and discounts it by
/// uncertainty. It never writes objects, Delta, Attention, or Workspace.
#[inline]
pub fn evaluate(scenario: SpeculativeScenario) -> LocalComputeResult {
    let mut result = LocalComputeResult::empty();
    result.object_id = scenario.object_id;
    result.semantic_id = scenario.semantic_id;
    result.baseline_state = scenario.baseline_state;
    result.hypothetical_state = scenario.hypothetical_state;

    if scenario.object_id == 0
        || scenario.semantic_id == 0
        || scenario.hypothetical_state == scenario.baseline_state
        || scenario.support == 0
        || scenario.horizon > 1
    {
        return result;
    }

    let delta = if scenario.hypothetical_state >= scenario.baseline_state {
        scenario.hypothetical_state - scenario.baseline_state
    } else {
        scenario.baseline_state - scenario.hypothetical_state
    };

    let confidence = scenario.confidence_fixed.min(SCALE);
    let utility = ((delta as u64 * confidence as u64) / SCALE as u64) as u32;

    result.disposition = ComputeDisposition::Evaluated;
    result.delta = delta;
    result.confidence_fixed = confidence;
    result.utility_fixed = utility;
    result.work_used = 4;
    result
}

/// Evaluate a Workspace item by first converting its predicted transition
/// into the same bounded speculative representation used by REL-11.
#[inline]
pub fn evaluate_workspace(item: WorkspaceItem) -> LocalComputeResult {
    let scenario = SpeculativeScenario {
        kind: super::speculation::SpeculationKind::PredictedAlternative,
        object_id: item.object_id,
        semantic_id: item.semantic_id,
        context: item.context,
        baseline_state: item.current_state,
        hypothetical_state: item.predicted_state,
        previous_state: item.previous_state,
        support: item.support,
        confidence_fixed: SCALE.saturating_sub(item.uncertainty_fixed.min(SCALE)),
        uncertainty_fixed: item.uncertainty_fixed.min(SCALE),
        horizon: 1,
        timestamp: item.timestamp,
        provenance: item.provenance,
    };
    evaluate(scenario)
}

#[inline]
pub fn process(scenarios: &[SpeculativeScenario]) -> LocalComputeReport {
    let mut report = LocalComputeReport::empty();
    let mut index = 0usize;

    while index < scenarios.len() {
        if report.work_used.saturating_add(4) > MAX_WORK {
            report.stopped_by_budget = true;
            break;
        }

        let result = evaluate(scenarios[index]);
        report.work_used = report.work_used.saturating_add(4);

        match result.disposition {
            ComputeDisposition::Evaluated => {
                report.evaluated = report.evaluated.saturating_add(1);
                report.latest = result;
            }
            ComputeDisposition::Rejected | ComputeDisposition::BudgetExhausted => {
                report.rejected = report.rejected.saturating_add(1);
            }
        }

        index += 1;
    }

    report
}

#[inline]
pub fn self_test() -> bool {
    let scenario = SpeculativeScenario {
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
        provenance: super::relation::RelationProvenance {
            origin: 1,
            source: super::relation::ProvenanceSource::Local,
            revision: 1,
        },
    };

    let result = evaluate(scenario);
    result.disposition == ComputeDisposition::Evaluated
        && result.delta == 10
        && result.utility_fixed == 8
        && result.work_used == 4
}
