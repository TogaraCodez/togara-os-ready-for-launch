//! GNOSIS Speculative Engine — REL-11
//!
//! Bounded one-step counterfactual simulation overderived Conscious Workspace
//! signals.
//!
//! Contract:
//! - canonical GNOSIS state is never mutated;
//! - speculation consumes only derived Conscious Workspace observations;
//! - no speculative result becomes canonical truth;
//! - no actions are executed;
//! - no policy is selected;
//! - no semantic states are invented;
//! - every scenario preserves provenance and evidence;
//! - fixed-capacity storage only;
//! - deterministic ordering;
//! - integer-only arithmetic;
//! - bounded work;
//! - no heap allocation.
//!
//! Initial model:
//!
//!     WorkspaceItem
//!          |
//!          v
//!     predicted_state != current_state
//!          |
//!          v
//!     one-step hypothetical branch
//!          |
//!          v
//!     confidence / uncertainty / support
//!
//! This is deliberately a foundation rather than afull planner or simulator.
//! Multi-step simulation, branching search, speculative caching, dreaming,
//! planning, and execution belong to later layers.

use super::conscious::{self, WorkspaceItem};
use super::relation::RelationProvenance;
use super::semantic::{ContextId, ObjectId, SemanticId};
use spin::Mutex;

/// Fixed-point confidence/uncertainty scale.
pub const SCALE: u32 = 10_000;

/// Maximum speculative scenarios retained.
pub const MAX_SCENARIOS: usize = 16;

/// Maximum abstract workunits consumed by one speculative pass.
pub const MAX_WORK: usize = 256;

/// Maximum simulated horizon of the REL-11 foundation.
pub const MAX_HORIZON: u8 = 1;

/// Kind of bounded speculative branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpeculationKind {
    /// Hypothetically substitute the predicted state for the observed state.
    PredictedAlternative,
}

/// One bounded speculative scenario.
///
/// Every field is derived from an existing Conscious Workspace item.
/// `hypothetical_state` is never written back intocanonical GNOSIS.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpeculativeScenario {
    pub kind: SpeculationKind,
    pub object_id: ObjectId,
    pub semantic_id: SemanticId,
    pub context: ContextId,

    /// State actually observed in the workspace.
    pub baseline_state: SemanticId,

    /// State hypothetically substituted for the observed state.
    pub hypothetical_state: SemanticId,

    /// Previous state supplied by the original derived observation.
    pub previous_state: SemanticId,

    /// Bounded evidence supporting the hypothetical outcome.
    pub support: u32,

    /// Fixed-point confidence inherited from the derived prediction.
    pub confidence_fixed: u32,

    /// Fixed-point uncertainty.
    pub uncertainty_fixed: u32,

    /// One-step simulation horizon.
    pub horizon: u8,

    pub timestamp: u64,
    pub provenance: RelationProvenance,
}

impl SpeculativeScenario {
    pub const fn empty() -> Self {
        Self {
            kind: SpeculationKind::PredictedAlternative,
            object_id: 0,
            semantic_id: 0,
            context: 0,
            baseline_state: 0,
            hypothetical_state: 0,
            previous_state: 0,
            support: 0,
            confidence_fixed: 0,
            uncertainty_fixed: SCALE,
            horizon: 0,
            timestamp: 0,
            provenance: RelationProvenance {
                origin: 0,
                source: super::relation::ProvenanceSource::Local,
                revision: 0,
            },
        }
    }
}

#[derive(Clone, Copy)]
struct SpeculationState {
    scenarios: [SpeculativeScenario; MAX_SCENARIOS],
    used: usize,
    cycle: u64,
}

impl SpeculationState {
    const fn empty() -> Self {
        Self {
            scenarios: [SpeculativeScenario::empty(); MAX_SCENARIOS],
            used: 0,
            cycle: 0,
        }
    }
}

static ENGINE: Mutex<SpeculationState> = Mutex::new(SpeculationState::empty());

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpeculationReport {
    pub workspace_observed: usize,
    pub scenarios_generated: usize,
    pub scenarios_rejected: usize,
    pub duplicates: usize,

    pub work_used: usize,
    pub work_budget: usize,
    pub stopped_by_budget: bool,
    pub overflow: bool,

    pub latest: SpeculativeScenario,
}

impl SpeculationReport {
    const fn empty() -> Self {
        Self {
            workspace_observed: 0,
            scenarios_generated: 0,
            scenarios_rejected: 0,
            duplicates: 0,
            work_used: 0,
            work_budget: MAX_WORK,
            stopped_by_budget: false,
            overflow: false,
            latest: SpeculativeScenario::empty(),
        }
    }
}

#[inline]
fn charge(report: &mut SpeculationReport, amount: usize) -> bool {
    let next = report.work_used.saturating_add(amount);

    if next > report.work_budget {
        report.work_used = report.work_budget;
        report.stopped_by_budget = true;
        report.overflow = true;
        return false;
    }

    report.work_used = next;
    true
}

/// Deterministic total ordering for speculative scenarios.
///
/// Higher confidence wins.
/// Lower uncertainty wins.
/// Higher support wins.
/// Newer observations win.
/// Remaining fields provide stable identity ordering.
#[inline]
fn outranks(a: &SpeculativeScenario, b: &SpeculativeScenario) -> bool {
    if a.confidence_fixed != b.confidence_fixed {
        return a.confidence_fixed > b.confidence_fixed;
    }

    if a.uncertainty_fixed != b.uncertainty_fixed {
        return a.uncertainty_fixed < b.uncertainty_fixed;
    }

    if a.support != b.support {
        return a.support > b.support;
    }

    if a.timestamp != b.timestamp {
        return a.timestamp > b.timestamp;
    }

    if a.object_id != b.object_id {
        return a.object_id < b.object_id;
    }

    if a.semantic_id != b.semantic_id {
        return a.semantic_id < b.semantic_id;
    }

    if a.context != b.context {
        return a.context < b.context;
    }

    if a.baseline_state != b.baseline_state {
        return a.baseline_state < b.baseline_state;
    }

    if a.hypothetical_state != b.hypothetical_state {
        return a.hypothetical_state < b.hypothetical_state;
    }

    a.previous_state < b.previous_state
}

#[inline]
fn same_scenario(a: &SpeculativeScenario, b: &SpeculativeScenario) -> bool {
    a.kind == b.kind
        && a.object_id == b.object_id
        && a.semantic_id == b.semantic_id
        && a.context == b.context
        && a.baseline_state == b.baseline_state
        && a.hypothetical_state == b.hypothetical_state
        && a.previous_state == b.previous_state
        && a.timestamp == b.timestamp
        && a.provenance == b.provenance
}

#[inline]
fn contains_scenario(state: &SpeculationState, scenario: &SpeculativeScenario) -> bool {
    let mut index = 0usize;

    while index < state.used {
        if same_scenario(&state.scenarios[index], scenario) {
            return true;
        }

        index += 1;
    }

    false
}

/// Convert one derived workspace item into a one-step hypothetical branch.
///
/// A branch is only generated when the workspace already contains a distinct
/// predicted state. No alternative state is invented here.
#[inline]
fn from_workspace(item: &WorkspaceItem) -> Option<SpeculativeScenario> {
    if item.object_id == 0
        || item.semantic_id == 0
        || item.predicted_state == 0
        || item.predicted_state == item.current_state
        || item.support == 0
    {
        return None;
    }

    let confidence = SCALE.saturating_sub(item.uncertainty_fixed.min(SCALE));

    Some(SpeculativeScenario {
        kind: SpeculationKind::PredictedAlternative,
        object_id: item.object_id,
        semantic_id: item.semantic_id,
        context: item.context,
        baseline_state: item.current_state,
        hypothetical_state: item.predicted_state,
        previous_state: item.previous_state,
        support: item.support,
        confidence_fixed: confidence,
        uncertainty_fixed: item.uncertainty_fixed.min(SCALE),
        horizon: MAX_HORIZON,
        timestamp: item.timestamp,
        provenance: item.provenance,
    })
}

#[inline]
fn admit(state: &mut SpeculationState, scenario: SpeculativeScenario) -> bool {
    if state.used < MAX_SCENARIOS {
        state.scenarios[state.used] = scenario;
        state.used += 1;
        return true;
    }

    let mut worst = 0usize;
    let mut index = 1usize;

    while index < MAX_SCENARIOS {
        if outranks(&state.scenarios[worst], &state.scenarios[index]) {
            worst = index;
        }

        index += 1;
    }

    if outranks(&scenario, &state.scenarios[worst]) {
        state.scenarios[worst] = scenario;
        true
    } else {
        false
    }
}

/// Process the current bounded Conscious Workspace.
///
/// The workspace is observed read-only. No canonical or Conscious state is
/// mutated by this engine.
#[inline]
pub fn process() -> SpeculationReport {
    let mut report = SpeculationReport::empty();

    {
        let mut state = ENGINE.lock();
        state.cycle = state.cycle.saturating_add(1);
    }

    let mut workspace = [WorkspaceItem::empty(); conscious::MAX_WORKSPACE];
    let mut workspace_count = 0usize;

    conscious::for_each(|item| {
        if workspace_count < workspace.len() {
            workspace[workspace_count] = *item;
            workspace_count += 1;
        }
    });

    report.workspace_observed = workspace_count;

    let mut index = 0usize;

    while index < workspace_count {
        if !charge(&mut report, 4) {
            break;
        }

        let item = workspace[index];

        let Some(scenario) = from_workspace(&item) else {
            report.scenarios_rejected = report.scenarios_rejected.saturating_add(1);
            index += 1;
            continue;
        };

        let mut state = ENGINE.lock();

        if contains_scenario(&state, &scenario) {
            report.duplicates = report.duplicates.saturating_add(1);
            index += 1;
            continue;
        }

        if admit(&mut state, scenario) {
            report.scenarios_generated = report.scenarios_generated.saturating_add(1);
            report.latest = scenario;
        } else {
            report.scenarios_rejected = report.scenarios_rejected.saturating_add(1);
        }

        index += 1;
    }

    report
}

/// Returns the number ofretained speculative scenarios.
#[inline]
pub fn count() -> usize {
    ENGINE.lock().used
}

/// Visits retained scenarios in deterministic ranking order without consuming
/// or mutating the speculative engine.
///
/// The engine lock is released before invoking the caller callback. This is
/// required so callbacks may safely call `count()`, `focus()`, `clear()`, or
/// other speculation APIs without recursive-lock deadlock.
pub fn for_each<F: FnMut(&SpeculativeScenario)>(mut callback: F) {
    let mut snapshot = [SpeculativeScenario::empty(); MAX_SCENARIOS];
    let mut count = 0usize;

    {
        let state = ENGINE.lock();

        let mut used = [false; MAX_SCENARIOS];
        let mut rank = 0usize;

        while rank < state.used {
            let mut selected: Option<usize> = None;
            let mut index = 0usize;

            while index < state.used {
                if !used[index] {
                    match selected {
                        None => selected = Some(index),
                        Some(current) => {
                            if outranks(&state.scenarios[index], &state.scenarios[current]) {
                                selected = Some(index);
                            }
                        }
                    }
                }

                index += 1;
            }

            if let Some(index) = selected {
                used[index] = true;
                snapshot[count] = state.scenarios[index];
                count += 1;
            }

            rank += 1;
        }
    }

    let mut index = 0usize;

    while index < count {
        callback(&snapshot[index]);
        index += 1;
    }
}

/// Returns the highest-ranked retained scenario.
#[inline]
pub fn focus() -> SpeculativeScenario {
    let state = ENGINE.lock();

    if state.used == 0 {
        return SpeculativeScenario::empty();
    }

    let mut best = 0usize;
    let mut index = 1usize;

    while index < state.used {
        if outranks(&state.scenarios[index], &state.scenarios[best]) {
            best = index;
        }

        index += 1;
    }

    state.scenarios[best]
}

/// Clears only speculative derived state.
#[inline]
pub fn clear() {
    *ENGINE.lock() = SpeculationState::empty();
}

/// Deterministic REL-11 foundation self-test.
#[inline]
pub fn self_test() -> bool {
    clear();

    let item = WorkspaceItem {
        object_id: 42,
        semantic_id: super::semantic::ACTIVE,
        previous_state: super::semantic::READY,
        current_state: super::semantic::FAILED,
        predicted_state: super::semantic::ACTIVE,
        context: 7,
        salience_fixed: 9000,
        uncertainty_fixed: 2500,
        support: 3,
        timestamp: 100,
        provenance: super::relation::RelationProvenance {
            origin: 42,
            source: super::relation::ProvenanceSource::Local,
            revision: 9,
        },
        source: super::conscious::WorkspaceSource::Subconscious,
    };

    let scenario = match from_workspace(&item) {
        Some(value) => value,
        None => return false,
    };

    if scenario.object_id != 42
        || scenario.baseline_state != super::semantic::FAILED
        || scenario.hypothetical_state != super::semantic::ACTIVE
        || scenario.previous_state != super::semantic::READY
        || scenario.support != 3
        || scenario.confidence_fixed != 7500
        || scenario.uncertainty_fixed != 2500
        || scenario.horizon != 1
    {
        return false;
    }

    // Same observed and predicted state must not create speculation.
    let non_branching = WorkspaceItem {
        predicted_state: super::semantic::FAILED,
        ..item
    };

    if from_workspace(&non_branching).is_some() {
        return false;
    }

    // Zero evidence mustnot create speculation.
    let no_support = WorkspaceItem { support: 0, ..item };

    if from_workspace(&no_support).is_some() {
        return false;
    }

    // Invalid identity must not create speculation.
    let invalid = WorkspaceItem {
        object_id: 0,
        ..item
    };

    if from_workspace(&invalid).is_some() {
        return false;
    }

    let mut state = SpeculationState::empty();

    if !admit(&mut state, scenario) {
        return false;
    }

    if state.used != 1 || !contains_scenario(&state, &scenario) {
        return false;
    }

    // Duplicate suppression is deterministic.
    if contains_scenario(&state, &scenario) == false {
        return false;
    }

    // Ranking prefers confidence.
    let lower_confidence = SpeculativeScenario {
        confidence_fixed: 1000,
        uncertainty_fixed: 9000,
        support: 100,
        ..scenario
    };

    if !admit(&mut state, lower_confidence) {
        return false;
    }

    if outranks(&lower_confidence, &scenario) {
        return false;
    }

    // Work accounting must never exceed the hard bound.
    let mut report = SpeculationReport::empty();

    if !charge(&mut report, MAX_WORK) {
        return false;
    }

    if report.work_used != MAX_WORK {
        return false;
    }

    if charge(&mut report, 1) {
        return false;
    }

    if report.work_used != MAX_WORK || !report.stopped_by_budget {
        return false;
    }

    // Callback re-entrancy must not deadlock the engine.
    {
        let mut engine = ENGINE.lock();
        engine.scenarios[0] = scenario;
        engine.used = 1;
    }

    let mut visited = 0usize;

    for_each(|observed| {
        if observed.object_id == scenario.object_id {
            visited += 1;
        }

        // These calls must be safe while the callback is executing.
        let _ = count();
        let _ = focus();
    });

    if visited != 1 {
        clear();
        return false;
    }

    // Public state must remain independent from the local test state.
    clear();

    if count() != 0 {
        return false;
    }

    // No speculation is fabricated from an empty workspace.
    let empty_report = process();

    if empty_report.workspace_observed != 0 || empty_report.scenarios_generated != 0 || count() != 0
    {
        clear();
        return false;
    }

    clear();

    true
}
