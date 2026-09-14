//! Bounded GNOSIS cycle orchestration.
//!
//! The coordinator establishes the execution boundary between the existing
//! GNOSIS layers:
//!
//!     canonical Delta
//!           |
//!           v
//!       Attention
//!           |
//!           v
//!     Subconscious
//!           |
//!           v
//!       Conscious
//!
//! Canonical Delta remains authoritative. Attention receives observations
//! from a completed Delta snapshot, Subconscious remains the canonical FIFO
//! consumer, and Conscious observes only derived Attention/Subconscious state.

use super::attention::{self, AttentionError};
use super::conscious::{self, ConsciousReport};
use super::delta::{self, SemanticDelta};
use super::objects;
use super::relation::{ProvenanceSource, RelationProvenance};
use super::subconscious::{self, SubconsciousReport};
use super::transition::{self as transition_boundary, PublishError};

/// Maximum orchestration work units for the Delta -> Attention staging phase.
///
/// Downstream Subconscious and Conscious processing retain their own
/// independently bounded work budgets.
pub const MAX_WORK: usize = (delta::MAX_DELTAS * 2) + 2;

/// Maximum canonical Delta records staged by one orchestration cycle.
pub const MAX_DELTAS: usize = delta::MAX_DELTAS;

/// Result of one bounded GNOSIS orchestration cycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GnosisCycleReport {
    pub deltas_observed: usize,
    pub attention_admitted: usize,
    pub attention_rejected: usize,
    pub attention_full: usize,

    pub subconscious: SubconsciousReport,
    pub conscious: ConsciousReport,

    /// Work consumed by the coordinator itself.
    ///
    /// This excludes the independently bounded work reported by
    /// Subconscious and Conscious.
    pub orchestration_work_used: usize,
    pub orchestration_work_budget: usize,
    pub stopped_by_budget: bool,
}

impl GnosisCycleReport {
    #[inline]
    pub const fn empty() -> Self {
        Self {
            deltas_observed: 0,
            attention_admitted: 0,
            attention_rejected: 0,
            attention_full: 0,
            subconscious: SubconsciousReport::empty(),
            conscious: ConsciousReport::empty(),
            orchestration_work_used: 0,
            orchestration_work_budget: MAX_WORK,
            stopped_by_budget: false,
        }
    }
}

#[inline]
fn charge(report: &mut GnosisCycleReport, amount: usize) -> bool {
    let next = report.orchestration_work_used.saturating_add(amount);

    if next > MAX_WORK {
        report.stopped_by_budget = true;
        return false;
    }

    report.orchestration_work_used = next;
    true
}

/// Run one bounded GNOSIS processing cycle.
///
/// The cycle has four strictly ordered phases:
///
/// 1. Copy the canonical Delta ring into a caller-owned local snapshot.
/// 2. Reconstruct canonical FIFO order and stage those observations into
///    Attention after the snapshot is complete.
/// 3. Let Subconscious consume canonical Delta through `process_pending()`.
/// 4. Let Conscious observe Attention and Subconscious candidates through
///    its existing read-only boundaries.
///
/// The coordinator never directly mutates canonical Delta state.
#[inline]
pub fn process_cycle() -> GnosisCycleReport {
    let mut report = GnosisCycleReport::empty();

    // Phase 1: take a complete local copy before mutating Attention.
    //
    // `snapshot_fifo_into()` captures canonical FIFO order atomically under
    // the Delta queue lock, so no separate HEAD/COUNT observation is needed.
    let mut snapshot = [None; MAX_DELTAS];

    let count = delta::snapshot_fifo_into(&mut snapshot).min(MAX_DELTAS);

    // Phase 2: stage the completed snapshot into Attention.
    //
    // The snapshot is already detached from Delta, so Attention mutation
    // cannot occur while a Delta observer is active.
    let mut offset = 0usize;

    while offset < count {
        if !charge(&mut report, 1) {
            return report;
        }

        if let Some(delta) = snapshot[offset] {
            report.deltas_observed = report.deltas_observed.saturating_add(1);

            if !charge(&mut report, 1) {
                return report;
            }

            match attention::push(delta) {
                Ok(_) => {
                    report.attention_admitted = report.attention_admitted.saturating_add(1);
                }
                Err(AttentionError::Full) => {
                    report.attention_full = report.attention_full.saturating_add(1);
                }
                Err(AttentionError::Invalid) => {
                    report.attention_rejected = report.attention_rejected.saturating_add(1);
                }
            }
        }

        offset += 1;
    }

    // Phase 3: Subconscious is the canonical Delta consumer.
    //
    // Its own MAX_WORK bound remains authoritative and is reported
    // independently.
    report.subconscious = subconscious::process_pending();

    // Phase 4: Conscious observes the resulting derived state.
    //
    // Conscious owns its own MAX_WORK bound.
    report.conscious = conscious::process();

    report
}

/// Result of one canonical transition transaction attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionCycleResult {
    Published,
    AlreadyPublished,
    Invalid,
    Full,
    Busy,
}

/// Apply one canonical KOBJ transition and publish its immutable transition
/// record into the SemanticDelta boundary.
///
/// The Delta queue is preflighted before canonical mutation so a full queue
/// cannot create an unobservable canonical transition. Publication failures
/// after mutation are treated as impossible under the synchronous kernel
/// boundary; the result is still mapped explicitly rather than discarded.
#[inline]
pub fn transition_and_publish(
    object_id: super::semantic::ObjectId,
    new_state: super::semantic::SemanticId,
    timestamp: u64,
    origin: u64,
    revision: u64,
) -> TransitionCycleResult {
    if object_id == 0 || !super::semantic::is_valid(new_state) || revision == 0 {
        return TransitionCycleResult::Invalid;
    }

    let provenance = RelationProvenance {
        origin,
        source: ProvenanceSource::Local,
        revision,
    };

    match transition_boundary::transition_and_publish(object_id, new_state, timestamp, provenance) {
        Ok(transition_boundary::TransitionResult::Published(_)) => TransitionCycleResult::Published,
        Ok(transition_boundary::TransitionResult::AlreadyPublished(_)) => {
            TransitionCycleResult::AlreadyPublished
        }
        Err(PublishError::Invalid) => TransitionCycleResult::Invalid,
        Err(PublishError::Full) => TransitionCycleResult::Full,
        Err(PublishError::Busy) => TransitionCycleResult::Busy,
    }
}

/// GTEST-only reset.
///
/// This function clears global test fixtures, including canonical Objects and
/// Trust. It must run only in the isolated boot/shell GTEST context before
/// normal operational state is admitted.
#[inline]
fn clear_local_cognitive_loop_state() {
    attention::clear();
    transition_boundary::clear();
    delta::clear();
    conscious::clear();
    objects::clear();
    subconscious::clear();
    super::trust::clear();
}

/// Shared Local Cognitive Loop behavioral verifier.
///
/// This verifier is intentionally non-executing. It exercises the current
/// canonical transition, detached Delta snapshot, Attention, Subconscious,
/// Conscious Workspace, candidate freshness, and Trust authority boundary
/// using only source-verified APIs.
///
/// The normal `process_cycle()` path remains unchanged: it is the production
/// consumer of canonical Delta. This verifier specifically proves that the
/// same detached snapshot can be supplied to multiple derived consumers
/// without consuming the canonical queue.
#[inline]
pub fn local_cognitive_loop_gtest() -> bool {
    macro_rules! fail {
        () => {{
            clear_local_cognitive_loop_state();
            return false;
        }};
    }

    clear_local_cognitive_loop_state();

    let object = objects::KnowledgeObject {
        id: 0x7A0,
        semantic_type: super::semantic::STATE_OBJECT,
        state: super::semantic::READY,
        context: 77,
        flags: objects::FLAG_ACTIVE,
        version: objects::CURRENT_VERSION,
    };

    // 1. Canonical object begins READY.
    if !objects::insert(object) {
        fail!();
    }

    let mut initial_state = 0;
    let mut initial_version = 0u16;
    if objects::with_object(object.id, |value| {
        initial_state = value.state;
        initial_version = value.version;
    })
    .is_none()
        || initial_state != super::semantic::READY
        || initial_version != objects::CURRENT_VERSION
    {
        fail!();
    }

    // Prime the bounded Subconscious learner using detached, synthetic test
    // observations. These are test fixtures, not canonical Delta history.
    let training = [
        SemanticDelta {
            object_id: object.id,
            semantic_id: super::semantic::STATE_OBJECT,
            previous_state: super::semantic::READY,
            new_state: super::semantic::ACTIVE,
            context: object.context,
            timestamp: 100,
            provenance: RelationProvenance {
                origin: object.id,
                source: ProvenanceSource::Local,
                revision: u64::from(initial_version),
            },
        },
        SemanticDelta {
            object_id: object.id,
            semantic_id: super::semantic::STATE_OBJECT,
            previous_state: super::semantic::READY,
            new_state: super::semantic::ACTIVE,
            context: object.context,
            timestamp: 101,
            provenance: RelationProvenance {
                origin: object.id,
                source: ProvenanceSource::Local,
                revision: u64::from(initial_version),
            },
        },
        SemanticDelta {
            object_id: object.id,
            semantic_id: super::semantic::STATE_OBJECT,
            previous_state: super::semantic::READY,
            new_state: super::semantic::ACTIVE,
            context: object.context,
            timestamp: 102,
            provenance: RelationProvenance {
                origin: object.id,
                source: ProvenanceSource::Local,
                revision: u64::from(initial_version),
            },
        },
    ];

    let training_report = subconscious::process(&training);
    if training_report.valid_observations != training.len() {
        fail!();
    }

    // 2. Valid canonical READY -> ACTIVE transition.
    let expected_version = initial_version.saturating_add(1);
    if expected_version == initial_version || expected_version == 0 {
        fail!();
    }

    if transition_and_publish(
        object.id,
        super::semantic::ACTIVE,
        900,
        object.id,
        u64::from(expected_version),
    ) != TransitionCycleResult::Published
    {
        fail!();
    }

    // 3. Exactly one Delta and one publication exist.
    if delta::count() != 1 || transition_boundary::count() != 1 {
        fail!();
    }

    let mut snapshot = [None; MAX_DELTAS];
    let snapshot_count = delta::snapshot_fifo_into(&mut snapshot);

    // 4. One coherent detached FIFO snapshot exists.
    if snapshot_count != 1 || snapshot[0].is_none() {
        fail!();
    }

    let observed = match snapshot[0] {
        Some(value) => value,
        None => {
            fail!();
        }
    };

    if !observed.is_valid()
        || observed.object_id != object.id
        || observed.previous_state != super::semantic::READY
        || observed.new_state != super::semantic::ACTIVE
        || observed.context != object.context
        || observed.provenance.revision != u64::from(expected_version)
    {
        fail!();
    }

    // 5-6. The same detached snapshot goes to Attention and non-consuming
    // Subconscious processing. The canonical Delta queue remains unchanged.
    if attention::push(observed).is_err() {
        fail!();
    }

    let subconscious_report = subconscious::process(&[observed]);
    if subconscious_report.valid_observations != 1 || delta::count() != 1 {
        fail!();
    }

    // 7. Derived candidate/workspace path.
    let mut candidates = [subconscious::CandidateRecord::empty(); subconscious::MAX_CANDIDATES];
    let candidate_count = subconscious::latest_candidates(&mut candidates);
    let mut candidate_found = false;
    let mut candidate = subconscious::CandidateRecord::empty();

    let mut index = 0usize;
    while index < candidate_count {
        let value = candidates[index];
        if value.origin_object_id == object.id
            && value.timestamp == observed.timestamp
            && value.provenance.revision == u64::from(expected_version)
        {
            candidate_found = true;
            candidate = value;
            break;
        }
        index += 1;
    }

    if !candidate_found {
        fail!();
    }

    // 8. Derived processing leaves canonical state unchanged.
    let mut canonical_after_derived = objects::KnowledgeObject {
        id: 0,
        semantic_type: 0,
        state: 0,
        context: 0,
        flags: 0,
        version: 0,
    };

    if objects::with_object(object.id, |value| canonical_after_derived = *value).is_none()
        || canonical_after_derived.state != super::semantic::ACTIVE
        || canonical_after_derived.version != expected_version
    {
        fail!();
    }

    let conscious_report = conscious::process();
    if conscious_report.workspace_count == 0 {
        fail!();
    }

    if delta::count() != 1 {
        fail!();
    }

    if subconscious::candidate_freshness(&candidate) != subconscious::CandidateFreshness::Fresh {
        fail!();
    }

    // 9-10. A later canonical mutation makes the earlier candidate stale.
    let stale_version = expected_version.saturating_add(1);
    if stale_version == expected_version || stale_version == 0 {
        fail!();
    }

    if transition_and_publish(
        object.id,
        super::semantic::READY,
        901,
        object.id,
        u64::from(stale_version),
    ) != TransitionCycleResult::Published
    {
        fail!();
    }

    if subconscious::candidate_freshness(&candidate) != subconscious::CandidateFreshness::Stale {
        fail!();
    }

    // 11. Unknown authority is denied through the existing Trust boundary.
    let authority = super::trust::authorize(
        super::trust::AuthorizationRequest {
            principal_id: 0xFFFF,
            required_capability: 0,
            required_authority: 1,
            minimum_trust_fixed: 1,
        },
        None,
    );

    if authority != super::trust::AuthorizationDecision::NoPrincipal {
        fail!();
    }

    // 12. No execution API is invoked by this verifier.
    clear_local_cognitive_loop_state();

    true
}

/// Deterministic orchestration boundary self-test.
#[inline]
pub fn self_test() -> bool {
    // Run the shared Local Cognitive Loop behavioral verifier first.
    //
    // Both BIOS boot GTEST and shell GTEST reach this function through the
    // existing shared `run_gtest_chain()` path.
    if !local_cognitive_loop_gtest() {
        return false;
    }

    attention::clear();
    transition_boundary::clear();
    delta::clear();
    conscious::clear();
    objects::clear();

    let object = super::objects::KnowledgeObject {
        id: 0x700,
        semantic_type: super::semantic::STATE_OBJECT,
        state: super::semantic::READY,
        context: 77,
        flags: super::objects::FLAG_ACTIVE,
        version: super::objects::CURRENT_VERSION,
    };

    if !objects::insert(object) {
        objects::clear();
        transition_boundary::clear();
        delta::clear();
        attention::clear();
        conscious::clear();
        return false;
    }

    if transition_and_publish(object.id, super::semantic::ACTIVE, 900, 55, 1)
        != TransitionCycleResult::Published
    {
        objects::clear();
        transition_boundary::clear();
        delta::clear();
        attention::clear();
        conscious::clear();
        return false;
    }

    let mut canonical_state = 0;
    let mut canonical_version = 0;
    objects::with_object(object.id, |value| {
        canonical_state = value.state;
        canonical_version = value.version;
    });

    if canonical_state != super::semantic::ACTIVE
        || canonical_version <= object.version
        || delta::count() != 1
        || transition_boundary::count() != 1
    {
        objects::clear();
        transition_boundary::clear();
        delta::clear();
        attention::clear();
        conscious::clear();
        return false;
    }

    // A full Delta queue must reject before mutating the canonical object.
    delta::clear();
    transition_boundary::clear();
    objects::clear();

    if !objects::insert(object) {
        return false;
    }

    let mut fill = 0usize;
    while fill < delta::MAX_DELTAS {
        let delta_record = SemanticDelta {
            object_id: 10_000 + fill as u64,
            semantic_id: super::semantic::ACTIVE,
            previous_state: super::semantic::READY,
            new_state: super::semantic::ACTIVE,
            context: 1,
            timestamp: fill as u64 + 1,
            provenance: RelationProvenance {
                origin: 1,
                source: ProvenanceSource::Local,
                revision: fill as u64 + 1,
            },
        };

        if delta::push(delta_record).is_err() {
            objects::clear();
            transition_boundary::clear();
            delta::clear();
            attention::clear();
            conscious::clear();
            return false;
        }

        fill += 1;
    }

    if transition_and_publish(object.id, super::semantic::ACTIVE, 901, 55, 2)
        != TransitionCycleResult::Full
    {
        objects::clear();
        transition_boundary::clear();
        delta::clear();
        attention::clear();
        conscious::clear();
        return false;
    }

    let mut preserved_state = 0;
    objects::with_object(object.id, |value| preserved_state = value.state);

    if preserved_state != object.state {
        objects::clear();
        transition_boundary::clear();
        delta::clear();
        attention::clear();
        conscious::clear();
        return false;
    }

    delta::clear();
    transition_boundary::clear();
    objects::clear();

    let first = SemanticDelta {
        object_id: 0x11,
        semantic_id: super::semantic::ACTIVE,
        previous_state: super::semantic::INACTIVE,
        new_state: super::semantic::ACTIVE,
        context: 1,
        timestamp: 100,
        provenance: super::relation::RelationProvenance {
            origin: 1,
            source: super::relation::ProvenanceSource::Local,
            revision: 1,
        },
    };

    let second = SemanticDelta {
        object_id: 0x22,
        semantic_id: super::semantic::AVAILABLE,
        previous_state: super::semantic::UNAVAILABLE,
        new_state: super::semantic::AVAILABLE,
        context: 2,
        timestamp: 101,
        provenance: super::relation::RelationProvenance {
            origin: 2,
            source: super::relation::ProvenanceSource::Local,
            revision: 2,
        },
    };

    if delta::push(first).is_err() || delta::push(second).is_err() {
        delta::clear();
        attention::clear();
        conscious::clear();
        return false;
    }

    if delta::count() != 2 {
        delta::clear();
        attention::clear();
        conscious::clear();
        return false;
    }

    let report = process_cycle();

    // Both canonical deltas must have been observed and staged.
    if report.deltas_observed != 2
        || report.attention_admitted != 2
        || report.attention_rejected != 0
        || report.attention_full != 0
    {
        delta::clear();
        attention::clear();
        conscious::clear();
        return false;
    }

    // Subconscious owns canonical Delta consumption in the production cycle.
    if delta::count() != 0 {
        delta::clear();
        attention::clear();
        conscious::clear();
        return false;
    }

    // Attention must contain both derived records.
    if attention::count() != 2 {
        delta::clear();
        attention::clear();
        conscious::clear();
        return false;
    }

    let mut saw_first = false;
    let mut saw_second = false;

    attention::for_each(|record| {
        if record.object_id == first.object_id
            && record.semantic_id == first.semantic_id
            && record.previous_state == first.previous_state
            && record.new_state == first.new_state
            && record.context == first.context
            && record.timestamp == first.timestamp
        {
            saw_first = true;
        }

        if record.object_id == second.object_id
            && record.semantic_id == second.semantic_id
            && record.previous_state == second.previous_state
            && record.new_state == second.new_state
            && record.context == second.context
            && record.timestamp == second.timestamp
        {
            saw_second = true;
        }
    });

    if !saw_first || !saw_second {
        delta::clear();
        attention::clear();
        conscious::clear();
        return false;
    }

    // Conscious must have observed the staged Attention records.
    if report.conscious.workspace_count < 2 {
        delta::clear();
        attention::clear();
        conscious::clear();
        return false;
    }

    // Coordinator staging itself must remain within its explicit bound.
    if report.orchestration_work_used > MAX_WORK || report.orchestration_work_budget != MAX_WORK {
        delta::clear();
        attention::clear();
        conscious::clear();
        return false;
    }

    // Downstream layers retain their own independent bounds.
    if report.subconscious.work_used > subconscious::MAX_WORK
        || report.conscious.work_used > conscious::MAX_WORK
    {
        delta::clear();
        attention::clear();
        conscious::clear();
        return false;
    }

    delta::clear();
    attention::clear();
    conscious::clear();
    transition_boundary::clear();
    objects::clear();

    true
}
