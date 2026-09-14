//! GNOSIS Semantic Subconscious Engine — REL-09
//!
//! Bounded, deterministic, integer-only background cognition over
//! SemanticDelta observations.
//!
//! REL-09 provides:
//! - bounded transition learning
//! - repeated-event / pattern detection
//! - one-step categorical prediction
//! - prediction error tracking
//! - bounded anomaly detection
//! - provisional candidate generation
//! - fixed-point confidence / uncertainty
//! - semantic event compression
//! - bounded resource-state observation through semantic transitions
//! - deterministic provenance preservation
//! - bounded duplicate suppression
//! - explicit CPU/work budgeting
//!
//! The Subconscious is a DERIVED observer.
//!
//! It MUST NOT:
//! - mutate canonical GNOSIS state
//! - mutate Knowledge, Relations, Graph, Messaging, Attention, Body, Soul,
//!   Spirit, or Delta state
//! - turn predictions into facts
//! - execute actions
//! - make final decisions
//! - act as policy
//! - perform unbounded computation
//!
//! Mathematical contract:
//!
//!     D_s = f_sub(D)
//!
//! where |D_s| << |D| for sufficiently repetitive input.
//!
//! REL-09 deliberately uses deterministic bounded statistical learning rather
//! than neural networks, floating point, embeddings, or external ML.
//!
//! Learning model:
//!
//!     (object_id, semantic_id, previous_state, context) -> next_state
//!
//! Prediction:
//!
//!     P(next_state | object_id, semantic_id, previous_state, context)
//!
//! is represented by bounded transition support.
//!
//! Confidence:
//!
//!     C = hits / attempts
//!
//! using fixed-point SCALE = 10000.
//!
//! Uncertainty:
//!
//!     U = SCALE - C
//!
//! Prediction and anomaly records remain provisional and preserve provenance.
//!
//! Speculative computation, speculative cache, memory consolidation, dreaming,
//! multi-step models, and distributed cognition remain separate future layers.

use spin::Mutex;

use super::delta::SemanticDelta;
use super::objects;
use super::relation::{ProvenanceSource, RelationProvenance};
use super::semantic::{ContextId, ObjectId, SemanticId};

/// Fixed-point confidence / uncertainty scale.
pub const SCALE: u32 = 10_000;

/// Maximum learned transition classes.
pub const MAX_TRANSITIONS: usize = 32;

/// Maximum bounded state-frequency classes.
pub const MAX_STATE_COUNTS: usize = 32;

/// Maximum retained provisional predictions.
pub const MAX_PREDICTIONS: usize = 16;

/// Maximum retained anomaly records.
pub const MAX_ANOMALIES: usize = 16;

/// Maximum retained candidate records.
pub const MAX_CANDIDATES: usize = 16;

/// Maximum semantic pattern records.
pub const MAX_PATTERNS: usize = 16;

/// Maximum observations accepted by one explicit processing call.
pub const MAX_OBSERVATIONS: usize = 64;

/// Maximum bounded exact-once window.
pub const MAX_SEEN: usize = 128;

/// Maximum abstract work units per processing cycle.
pub const MAX_WORK: usize = 4096;

/// Minimum transition evidence required before prediction becomes eligible.
pub const MIN_EVIDENCE: u32 = 3;

/// Maximum percentage representation.
pub const MAX_PERCENT: u32 = 100;

/// Maximum bounded surprisal representation.
pub const MAX_SURPRISAL_BITS: u8 = 63;

/// Pattern is considered recurrent after this many observations.
pub const MIN_PATTERN_EVIDENCE: u32 = 3;

/// Candidate kinds produced by the Subconscious.
///
/// These are signals only. They do not represent commands or decisions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateKind {
    RepeatedTransition,
    Prediction,
    Anomaly,
    ResourceTransition,
}

/// Result of evaluating a provisional prediction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PredictionOutcome {
    Unresolved,
    Hit,
    Miss,
}

/// Classification of a derived anomaly observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnomalyState {
    Unknown,
    Expected,
    Anomalous,
}

/// Bounded learned transition.
///
/// Learning key:
///
///     (object_id, semantic_id, previous_state, next_state, context)
///
/// Object identity is part of the first-order learning key so independent
/// entities cannot contaminate one another's transition distributions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransitionStats {
    pub object_id: ObjectId,
    pub semantic_id: SemanticId,
    pub previous_state: SemanticId,
    pub next_state: SemanticId,
    pub context: ContextId,
    pub count: u32,
    pub prediction_attempts: u32,
    pub prediction_hits: u32,
    pub prediction_misses: u32,
    pub last_seen: u64,
    pub provenance: RelationProvenance,
}

impl TransitionStats {
    #[inline]
    pub const fn empty() -> Self {
        Self {
            object_id: 0,
            semantic_id: 0,
            previous_state: 0,
            next_state: 0,
            context: 0,
            count: 0,
            prediction_attempts: 0,
            prediction_hits: 0,
            prediction_misses: 0,
            last_seen: 0,
            provenance: RelationProvenance {
                origin: 0,
                source: ProvenanceSource::Local,
                revision: 0,
            },
        }
    }
}

/// Bounded occurrence statistics.
///
/// This supports repeated-state detection without becoming canonical memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateCount {
    pub object_id: ObjectId,
    pub semantic_id: SemanticId,
    pub state: SemanticId,
    pub context: ContextId,
    pub count: u32,
}

impl StateCount {
    #[inline]
    pub const fn empty() -> Self {
        Self {
            object_id: 0,
            semantic_id: 0,
            state: 0,
            context: 0,
            count: 0,
        }
    }
}

/// Recurrent transition pattern.
///
/// A pattern is a statistical observation of recurrence, not semantic truth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PatternRecord {
    pub object_id: ObjectId,
    pub semantic_id: SemanticId,
    pub previous_state: SemanticId,
    pub next_state: SemanticId,
    pub context: ContextId,
    pub support: u32,
    pub first_seen: u64,
    pub last_seen: u64,
    pub recurrence: u32,
    pub provenance: RelationProvenance,
}

impl PatternRecord {
    #[inline]
    pub const fn empty() -> Self {
        Self {
            object_id: 0,
            semantic_id: 0,
            previous_state: 0,
            next_state: 0,
            context: 0,
            support: 0,
            first_seen: 0,
            last_seen: 0,
            recurrence: 0,
            provenance: RelationProvenance {
                origin: 0,
                source: ProvenanceSource::Local,
                revision: 0,
            },
        }
    }
}

/// Provisional one-step prediction.
///
/// Identity:
///
///     (semantic_id, source_state, context)
///
/// Only one unresolved prediction for a given identity is retained.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PredictionRecord {
    pub semantic_id: SemanticId,
    pub source_state: SemanticId,
    pub target_state: SemanticId,
    pub context: ContextId,
    pub support: u32,
    pub total_outgoing: u32,
    pub confidence_fixed: u32,
    pub uncertainty_fixed: u32,
    pub created_at: u64,
    pub outcome: PredictionOutcome,
    pub error: u32,
    pub provenance: RelationProvenance,
    pub origin_object_id: ObjectId,
}

impl PredictionRecord {
    #[inline]
    pub const fn empty() -> Self {
        Self {
            semantic_id: 0,
            source_state: 0,
            target_state: 0,
            context: 0,
            support: 0,
            total_outgoing: 0,
            confidence_fixed: 0,
            uncertainty_fixed: SCALE,
            created_at: 0,
            outcome: PredictionOutcome::Unresolved,
            error: 0,
            provenance: RelationProvenance {
                origin: 0,
                source: ProvenanceSource::Local,
                revision: 0,
            },
            origin_object_id: 0,
        }
    }
}

/// Derived anomaly record.
///
/// Anomaly detection is statistical only. An anomalous transition is not
/// automatically invalid, unsafe, or forbidden.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnomalyRecord {
    pub semantic_id: SemanticId,
    pub source_state: SemanticId,
    pub actual_state: SemanticId,
    pub expected_state: SemanticId,
    pub context: ContextId,
    pub score_fixed: u32,
    pub baseline: u32,
    pub state: AnomalyState,
    pub provenance: RelationProvenance,
    pub origin_object_id: ObjectId,
}

impl AnomalyRecord {
    #[inline]
    pub const fn empty() -> Self {
        Self {
            semantic_id: 0,
            source_state: 0,
            actual_state: 0,
            expected_state: 0,
            context: 0,
            score_fixed: 0,
            baseline: 0,
            state: AnomalyState::Unknown,
            provenance: RelationProvenance {
                origin: 0,
                source: ProvenanceSource::Local,
                revision: 0,
            },
            origin_object_id: 0,
        }
    }
}

/// Provisional Subconscious candidate.
///
/// Candidates are derived work products and never commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidateRecord {
    pub kind: CandidateKind,
    pub semantic_id: SemanticId,
    pub previous_state: SemanticId,
    pub observed_state: SemanticId,
    pub predicted_state: SemanticId,
    pub context: ContextId,
    pub score_fixed: u32,
    pub uncertainty_fixed: u32,
    pub support: u32,
    pub timestamp: u64,
    pub provenance: RelationProvenance,
    pub origin_object_id: ObjectId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateFreshness {
    Fresh,
    Stale,
    MissingCanonical,
    InvalidProvenance,
}

/// Validate a derived candidate against the current canonical object version.
///
/// For candidates produced from canonical GNOSIS transitions,
/// `provenance.revision` is the canonical object version captured by the
/// producer. A later canonical transition increments that version and makes
/// the older candidate stale. This function observes canonical state only; it
/// never mutates it.
#[inline]
pub fn candidate_freshness(candidate: &CandidateRecord) -> CandidateFreshness {
    if candidate.origin_object_id == 0 || candidate.provenance.revision == 0 {
        return CandidateFreshness::InvalidProvenance;
    }

    let mut current_version = 0u16;
    let found = objects::with_object(candidate.origin_object_id, |object| {
        current_version = object.version;
    })
    .is_some();

    if !found {
        return CandidateFreshness::MissingCanonical;
    }

    if u64::from(current_version) == candidate.provenance.revision {
        CandidateFreshness::Fresh
    } else {
        CandidateFreshness::Stale
    }
}

impl CandidateRecord {
    #[inline]
    pub const fn empty() -> Self {
        Self {
            kind: CandidateKind::RepeatedTransition,
            semantic_id: 0,
            previous_state: 0,
            observed_state: 0,
            predicted_state: 0,
            context: 0,
            score_fixed: 0,
            uncertainty_fixed: SCALE,
            support: 0,
            timestamp: 0,
            provenance: RelationProvenance {
                origin: 0,
                source: ProvenanceSource::Local,
                revision: 0,
            },
            origin_object_id: 0,
        }
    }
}

/// Aggregate bounded report.
///
/// All fields are derived. `provisional` is true whenever processing stopped
/// before the requested input was fully handled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubconsciousReport {
    pub observations_seen: usize,
    pub valid_observations: usize,
    pub invalid_observations: usize,
    pub duplicate_observations: usize,

    pub transition_count: usize,
    pub state_count: usize,
    pub pattern_count: usize,

    pub prediction_count: usize,
    pub hit_count: usize,
    pub miss_count: usize,

    pub anomaly_count: usize,
    pub candidate_count: usize,

    pub raw_units: u32,
    pub summary_units: u32,
    pub compression_x100: u32,

    pub information_available: bool,
    pub latest_surprisal_bits: u8,

    pub work_used: usize,
    pub work_budget: usize,
    pub overflow: bool,
    pub stopped_by_budget: bool,

    pub provisional: bool,

    pub latest_prediction: PredictionRecord,
    pub latest_anomaly: AnomalyRecord,
    pub latest_candidate: CandidateRecord,
}

impl SubconsciousReport {
    #[inline]
    pub const fn empty() -> Self {
        Self {
            observations_seen: 0,
            valid_observations: 0,
            invalid_observations: 0,
            duplicate_observations: 0,
            transition_count: 0,
            state_count: 0,
            pattern_count: 0,
            prediction_count: 0,
            hit_count: 0,
            miss_count: 0,
            anomaly_count: 0,
            candidate_count: 0,
            raw_units: 0,
            summary_units: 0,
            compression_x100: 0,
            information_available: false,
            latest_surprisal_bits: 0,
            work_used: 0,
            work_budget: MAX_WORK,
            overflow: false,
            stopped_by_budget: false,
            provisional: true,
            latest_prediction: PredictionRecord::empty(),
            latest_anomaly: AnomalyRecord::empty(),
            latest_candidate: CandidateRecord::empty(),
        }
    }
}

/// Exact event identity used for bounded duplicate suppression.
#[derive(Clone, Copy, PartialEq, Eq)]
struct DeltaEventKey {
    object_id: ObjectId,
    semantic_id: SemanticId,
    previous_state: SemanticId,
    new_state: SemanticId,
    context: ContextId,
    timestamp: u64,
    provenance_origin: ObjectId,
    provenance_source: ProvenanceSource,
    provenance_revision: u64,
}

impl DeltaEventKey {
    #[inline]
    fn from_delta(delta: &SemanticDelta) -> Self {
        Self {
            object_id: delta.object_id,
            semantic_id: delta.semantic_id,
            previous_state: delta.previous_state,
            new_state: delta.new_state,
            context: delta.context,
            timestamp: delta.timestamp,
            provenance_origin: delta.provenance.origin,
            provenance_source: delta.provenance.source,
            provenance_revision: delta.provenance.revision,
        }
    }
}

/// Complete derived Subconscious state.
#[derive(Clone, Copy)]
struct EngineState {
    transitions: [TransitionStats; MAX_TRANSITIONS],
    state_counts: [StateCount; MAX_STATE_COUNTS],
    patterns: [PatternRecord; MAX_PATTERNS],
    predictions: [PredictionRecord; MAX_PREDICTIONS],
    anomalies: [AnomalyRecord; MAX_ANOMALIES],
    candidates: [CandidateRecord; MAX_CANDIDATES],
    seen_keys: [DeltaEventKey; MAX_SEEN],

    transition_used: usize,
    state_used: usize,
    pattern_used: usize,
    prediction_used: usize,
    anomaly_used: usize,
    candidate_used: usize,
    seen_used: usize,

    observation_counter: u64,
}

impl EngineState {
    const fn empty() -> Self {
        Self {
            transitions: [TransitionStats::empty(); MAX_TRANSITIONS],
            state_counts: [StateCount::empty(); MAX_STATE_COUNTS],
            patterns: [PatternRecord::empty(); MAX_PATTERNS],
            predictions: [PredictionRecord::empty(); MAX_PREDICTIONS],
            anomalies: [AnomalyRecord::empty(); MAX_ANOMALIES],
            candidates: [CandidateRecord::empty(); MAX_CANDIDATES],
            seen_keys: [DeltaEventKey {
                object_id: 0,
                semantic_id: 0,
                previous_state: 0,
                new_state: 0,
                context: 0,
                timestamp: 0,
                provenance_origin: 0,
                provenance_source: ProvenanceSource::Local,
                provenance_revision: 0,
            }; MAX_SEEN],
            transition_used: 0,
            state_used: 0,
            pattern_used: 0,
            prediction_used: 0,
            anomaly_used: 0,
            candidate_used: 0,
            seen_used: 0,
            observation_counter: 0,
        }
    }
}

static ENGINE: Mutex<EngineState> = Mutex::new(EngineState::empty());

/// Charge bounded deterministic work.
///
/// Work is intentionally counted at primitive bounded-operation level rather
/// than pretending that a complete array lookup costs one unit.
#[inline]
fn charge(report: &mut SubconsciousReport, cost: usize) -> bool {
    let next = report.work_used.saturating_add(cost);
    if next > report.work_budget {
        report.overflow = true;
        report.stopped_by_budget = true;
        return false;
    }

    report.work_used = next;
    true
}

/// Fixed-point confidence in 0..=SCALE.
#[inline]
fn confidence_fixed(successes: u32, attempts: u32) -> u32 {
    if attempts == 0 {
        return 0;
    }

    let capped = successes.min(attempts);
    let numerator = u64::from(capped).saturating_mul(u64::from(SCALE));

    numerator
        .saturating_div(u64::from(attempts))
        .min(u64::from(SCALE)) as u32
}

/// Fixed-point uncertainty complementary to confidence.
#[inline]
fn uncertainty_fixed(confidence: u32) -> u32 {
    SCALE.saturating_sub(confidence.min(SCALE))
}

/// Categorical prediction error.
///
/// This deliberately does not pretend that semantic IDs have metric distance.
#[inline]
fn categorical_error(expected: SemanticId, actual: SemanticId) -> u32 {
    if expected == actual { 0 } else { 1 }
}

/// Integer-only bounded surprisal approximation.
///
/// This reports a conservative integer bit bucket rather than floating-point
/// information theory.
#[inline]
fn information_surprisal_bits(count: u32, total: u32) -> u8 {
    if count == 0 || total == 0 {
        return MAX_SURPRISAL_BITS;
    }

    let mut ratio = u64::from(total) / u64::from(count);

    if u64::from(total) % u64::from(count) != 0 {
        ratio = ratio.saturating_add(1);
    }

    if ratio <= 1 {
        return 0;
    }

    let mut bits = 0u8;
    let mut threshold = 1u64;

    while threshold < ratio && bits < MAX_SURPRISAL_BITS {
        threshold = threshold.saturating_mul(2);
        bits = bits.saturating_add(1);
    }

    bits
}

#[inline]
fn find_seen_key(
    engine: &EngineState,
    key: &DeltaEventKey,
    report: &mut SubconsciousReport,
) -> Option<bool> {
    let mut index = 0usize;

    while index < engine.seen_used {
        if !charge(report, 1) {
            return None;
        }

        if engine.seen_keys[index] == *key {
            return Some(true);
        }

        index += 1;
    }

    Some(false)
}

#[inline]
fn insert_seen_key(engine: &mut EngineState, key: DeltaEventKey) {
    if engine.seen_used < MAX_SEEN {
        engine.seen_keys[engine.seen_used] = key;
        engine.seen_used += 1;
        return;
    }

    let mut index = 1usize;

    while index < MAX_SEEN {
        engine.seen_keys[index - 1] = engine.seen_keys[index];
        index += 1;
    }

    engine.seen_keys[MAX_SEEN - 1] = key;
}

#[inline]
fn find_transition_slot(
    engine: &EngineState,
    object_id: ObjectId,
    semantic_id: SemanticId,
    previous_state: SemanticId,
    next_state: SemanticId,
    context: ContextId,
    report: &mut SubconsciousReport,
) -> Option<usize> {
    let mut index = 0usize;

    while index < engine.transition_used {
        if !charge(report, 1) {
            return None;
        }

        let entry = engine.transitions[index];

        if entry.object_id == object_id
            && entry.semantic_id == semantic_id
            && entry.previous_state == previous_state
            && entry.next_state == next_state
            && entry.context == context
        {
            return Some(index);
        }

        index += 1;
    }

    None
}

#[inline]
fn find_transition_slot_unbudgeted(
    engine: &EngineState,
    object_id: ObjectId,
    semantic_id: SemanticId,
    previous_state: SemanticId,
    next_state: SemanticId,
    context: ContextId,
) -> Option<usize> {
    let mut index = 0usize;

    while index < engine.transition_used {
        let entry = engine.transitions[index];

        if entry.object_id == object_id
            && entry.semantic_id == semantic_id
            && entry.previous_state == previous_state
            && entry.next_state == next_state
            && entry.context == context
        {
            return Some(index);
        }

        index += 1;
    }

    None
}

/// Replace the least-supported transition.
///
/// Ties are resolved deterministically by oldest timestamp, then lowest slot.
#[inline]
fn insert_transition_slot(engine: &mut EngineState, entry: TransitionStats) -> usize {
    if engine.transition_used < MAX_TRANSITIONS {
        let index = engine.transition_used;
        engine.transitions[index] = entry;
        engine.transition_used += 1;
        return index;
    }

    let mut replace_index = 0usize;
    let mut index = 1usize;

    while index < MAX_TRANSITIONS {
        let current = engine.transitions[index];
        let selected = engine.transitions[replace_index];

        if current.count < selected.count
            || (current.count == selected.count && current.last_seen < selected.last_seen)
        {
            replace_index = index;
        }

        index += 1;
    }

    engine.transitions[replace_index] = entry;
    replace_index
}

#[inline]
fn find_state_slot(
    engine: &EngineState,
    object_id: ObjectId,
    semantic_id: SemanticId,
    state: SemanticId,
    context: ContextId,
    report: &mut SubconsciousReport,
) -> Option<usize> {
    let mut index = 0usize;

    while index < engine.state_used {
        if !charge(report, 1) {
            return None;
        }

        let entry = engine.state_counts[index];

        if entry.object_id == object_id
            && entry.semantic_id == semantic_id
            && entry.state == state
            && entry.context == context
        {
            return Some(index);
        }

        index += 1;
    }

    None
}

#[inline]
fn insert_state_slot(engine: &mut EngineState, entry: StateCount) -> usize {
    if engine.state_used < MAX_STATE_COUNTS {
        let index = engine.state_used;
        engine.state_counts[index] = entry;
        engine.state_used += 1;
        return index;
    }

    let mut replace = 0usize;
    let mut index = 1usize;

    while index < MAX_STATE_COUNTS {
        if engine.state_counts[index].count < engine.state_counts[replace].count {
            replace = index;
        }

        index += 1;
    }

    engine.state_counts[replace] = entry;
    replace
}

#[inline]
fn find_pattern_slot(
    engine: &EngineState,
    object_id: ObjectId,
    semantic_id: SemanticId,
    previous_state: SemanticId,
    next_state: SemanticId,
    context: ContextId,
    report: &mut SubconsciousReport,
) -> Option<usize> {
    let mut index = 0usize;

    while index < engine.pattern_used {
        if !charge(report, 1) {
            return None;
        }

        let entry = engine.patterns[index];

        if entry.object_id == object_id
            && entry.semantic_id == semantic_id
            && entry.previous_state == previous_state
            && entry.next_state == next_state
            && entry.context == context
        {
            return Some(index);
        }

        index += 1;
    }

    None
}

#[inline]
fn find_pattern_slot_unbudgeted(
    engine: &EngineState,
    object_id: ObjectId,
    semantic_id: SemanticId,
    previous_state: SemanticId,
    next_state: SemanticId,
    context: ContextId,
) -> Option<usize> {
    let mut index = 0usize;

    while index < engine.pattern_used {
        let entry = engine.patterns[index];

        if entry.object_id == object_id
            && entry.semantic_id == semantic_id
            && entry.previous_state == previous_state
            && entry.next_state == next_state
            && entry.context == context
        {
            return Some(index);
        }

        index += 1;
    }

    None
}

#[inline]
fn insert_pattern_slot(engine: &mut EngineState, entry: PatternRecord) -> usize {
    if engine.pattern_used < MAX_PATTERNS {
        let index = engine.pattern_used;
        engine.patterns[index] = entry;
        engine.pattern_used += 1;
        return index;
    }

    let mut replace = 0usize;
    let mut index = 1usize;

    while index < MAX_PATTERNS {
        if engine.patterns[index].support < engine.patterns[replace].support
            || (engine.patterns[index].support == engine.patterns[replace].support
                && engine.patterns[index].last_seen < engine.patterns[replace].last_seen)
        {
            replace = index;
        }

        index += 1;
    }

    engine.patterns[replace] = entry;
    replace
}

#[inline]
fn find_prediction_slot(
    engine: &EngineState,
    object_id: ObjectId,
    semantic_id: SemanticId,
    source_state: SemanticId,
    context: ContextId,
    report: &mut SubconsciousReport,
) -> Option<usize> {
    let mut index = 0usize;

    while index < engine.prediction_used {
        if !charge(report, 1) {
            return None;
        }

        let record = engine.predictions[index];

        if record.origin_object_id == object_id
            && record.semantic_id == semantic_id
            && record.source_state == source_state
            && record.context == context
        {
            return Some(index);
        }

        index += 1;
    }

    None
}

#[inline]
fn find_prediction_slot_unbudgeted(
    engine: &EngineState,
    object_id: ObjectId,
    semantic_id: SemanticId,
    source_state: SemanticId,
    context: ContextId,
) -> Option<usize> {
    let mut index = 0usize;

    while index < engine.prediction_used {
        let record = engine.predictions[index];

        if record.origin_object_id == object_id
            && record.semantic_id == semantic_id
            && record.source_state == source_state
            && record.context == context
        {
            return Some(index);
        }

        index += 1;
    }

    None
}

#[inline]
fn insert_prediction_slot(engine: &mut EngineState, entry: PredictionRecord) -> usize {
    if engine.prediction_used < MAX_PREDICTIONS {
        let index = engine.prediction_used;
        engine.predictions[index] = entry;
        engine.prediction_used += 1;
        return index;
    }

    let mut oldest_index = 0usize;
    let mut index = 1usize;

    while index < MAX_PREDICTIONS {
        if engine.predictions[index].created_at < engine.predictions[oldest_index].created_at {
            oldest_index = index;
        }

        index += 1;
    }

    engine.predictions[oldest_index] = entry;
    oldest_index
}

#[inline]
fn insert_anomaly_slot(engine: &mut EngineState, entry: AnomalyRecord) -> usize {
    if engine.anomaly_used < MAX_ANOMALIES {
        let index = engine.anomaly_used;
        engine.anomalies[index] = entry;
        engine.anomaly_used += 1;
        return index;
    }

    let mut oldest_index = 0usize;
    let mut index = 1usize;

    while index < MAX_ANOMALIES {
        if engine.anomalies[index].origin_object_id
            < engine.anomalies[oldest_index].origin_object_id
        {
            oldest_index = index;
        }

        index += 1;
    }

    engine.anomalies[oldest_index] = entry;
    oldest_index
}

#[inline]
fn insert_candidate_slot(engine: &mut EngineState, entry: CandidateRecord) -> usize {
    if engine.candidate_used < MAX_CANDIDATES {
        let index = engine.candidate_used;
        engine.candidates[index] = entry;
        engine.candidate_used += 1;
        return index;
    }

    let mut replace = 0usize;
    let mut index = 1usize;

    while index < MAX_CANDIDATES {
        if engine.candidates[index].score_fixed < engine.candidates[replace].score_fixed {
            replace = index;
        }

        index += 1;
    }

    engine.candidates[replace] = entry;
    replace
}

#[inline]
fn dominant_transition(
    engine: &EngineState,
    object_id: ObjectId,
    semantic_id: SemanticId,
    previous_state: SemanticId,
    context: ContextId,
) -> Option<TransitionStats> {
    let mut best = None;
    let mut best_count = 0u32;
    let mut best_target = u32::MAX;
    let mut index = 0usize;

    while index < engine.transition_used {
        let entry = engine.transitions[index];

        if entry.object_id == object_id
            && entry.semantic_id == semantic_id
            && entry.previous_state == previous_state
            && entry.context == context
        {
            if entry.count > best_count
                || (entry.count == best_count && entry.next_state < best_target)
            {
                best = Some(entry);
                best_count = entry.count;
                best_target = entry.next_state;
            }
        }

        index += 1;
    }

    best
}

#[inline]
fn total_outgoing_support(
    engine: &EngineState,
    object_id: ObjectId,
    semantic_id: SemanticId,
    previous_state: SemanticId,
    context: ContextId,
) -> u32 {
    let mut total = 0u32;
    let mut index = 0usize;

    while index < engine.transition_used {
        let entry = engine.transitions[index];

        if entry.object_id == object_id
            && entry.semantic_id == semantic_id
            && entry.previous_state == previous_state
            && entry.context == context
        {
            total = total.saturating_add(entry.count);
        }

        index += 1;
    }

    total
}

#[inline]
fn count_transition(
    engine: &EngineState,
    object_id: ObjectId,
    semantic_id: SemanticId,
    previous_state: SemanticId,
    next_state: SemanticId,
    context: ContextId,
) -> u32 {
    find_transition_slot_unbudgeted(
        engine,
        object_id,
        semantic_id,
        previous_state,
        next_state,
        context,
    )
    .map(|index| engine.transitions[index].count)
    .unwrap_or(0)
}

/// Evaluate and close a pending prediction before learning the current event.
///
/// This ordering is essential:
///
///     previous model -> prediction -> actual observation -> learning update
///
/// Otherwise the actual event could contaminate the prediction being evaluated.
#[inline]
fn evaluate_pending_prediction(
    engine: &mut EngineState,
    observation: &SemanticDelta,
    report: &mut SubconsciousReport,
) -> bool {
    let Some(slot) = find_prediction_slot(
        engine,
        observation.object_id,
        observation.semantic_id,
        observation.previous_state,
        observation.context,
        report,
    ) else {
        return true;
    };

    let mut pending = engine.predictions[slot];

    if pending.outcome != PredictionOutcome::Unresolved {
        return true;
    }

    let is_hit = pending.target_state == observation.new_state;

    pending.outcome = if is_hit {
        PredictionOutcome::Hit
    } else {
        PredictionOutcome::Miss
    };

    pending.error = categorical_error(pending.target_state, observation.new_state);
    pending.provenance = observation.provenance;
    pending.origin_object_id = observation.object_id;

    let transition_slot = find_transition_slot_unbudgeted(
        engine,
        pending.origin_object_id,
        pending.semantic_id,
        pending.source_state,
        pending.target_state,
        pending.context,
    );

    if let Some(transition_slot) = transition_slot {
        let mut stats = engine.transitions[transition_slot];

        stats.prediction_attempts = stats.prediction_attempts.saturating_add(1);

        if is_hit {
            stats.prediction_hits = stats.prediction_hits.saturating_add(1);
            report.hit_count = report.hit_count.saturating_add(1);
        } else {
            stats.prediction_misses = stats.prediction_misses.saturating_add(1);
            report.miss_count = report.miss_count.saturating_add(1);
        }

        engine.transitions[transition_slot] = stats;

        pending.support = stats.count;
        pending.total_outgoing = total_outgoing_support(
            engine,
            pending.origin_object_id,
            pending.semantic_id,
            pending.source_state,
            pending.context,
        );

        pending.confidence_fixed =
            confidence_fixed(stats.prediction_hits, stats.prediction_attempts);

        pending.uncertainty_fixed = uncertainty_fixed(pending.confidence_fixed);

        engine.predictions[slot] = pending;
        report.latest_prediction = pending;

        let candidate = CandidateRecord {
            kind: CandidateKind::Prediction,
            semantic_id: pending.semantic_id,
            previous_state: pending.source_state,
            observed_state: observation.new_state,
            predicted_state: pending.target_state,
            context: pending.context,
            score_fixed: pending.confidence_fixed,
            uncertainty_fixed: pending.uncertainty_fixed,
            support: pending.support,
            timestamp: observation.timestamp,
            provenance: observation.provenance,
            origin_object_id: observation.object_id,
        };

        insert_candidate_slot(engine, candidate);
        report.candidate_count = report.candidate_count.saturating_add(1);
        report.latest_candidate = candidate;
    } else {
        engine.predictions[slot] = pending;
        report.latest_prediction = pending;
    }

    true
}

/// Evaluate the current observation against the learned transition
/// distribution BEFORE inserting the current transition.
///
/// This prevents the current event from becoming evidence for itself.
#[inline]
fn evaluate_anomaly(engine: &EngineState, observation: &SemanticDelta) -> AnomalyRecord {
    let dominant = dominant_transition(
        engine,
        observation.object_id,
        observation.semantic_id,
        observation.previous_state,
        observation.context,
    );

    let Some(expected) = dominant else {
        return AnomalyRecord {
            semantic_id: observation.semantic_id,
            source_state: observation.previous_state,
            actual_state: observation.new_state,
            expected_state: 0,
            context: observation.context,
            score_fixed: 0,
            baseline: 0,
            state: AnomalyState::Unknown,
            provenance: observation.provenance,
            origin_object_id: observation.object_id,
        };
    };

    if expected.count < MIN_EVIDENCE {
        return AnomalyRecord {
            semantic_id: observation.semantic_id,
            source_state: observation.previous_state,
            actual_state: observation.new_state,
            expected_state: expected.next_state,
            context: observation.context,
            score_fixed: 0,
            baseline: expected.count,
            state: AnomalyState::Unknown,
            provenance: observation.provenance,
            origin_object_id: observation.object_id,
        };
    }

    let total = total_outgoing_support(
        engine,
        observation.object_id,
        observation.semantic_id,
        observation.previous_state,
        observation.context,
    );

    let actual_count = count_transition(
        engine,
        observation.object_id,
        observation.semantic_id,
        observation.previous_state,
        observation.new_state,
        observation.context,
    );

    let state = if expected.next_state == observation.new_state {
        AnomalyState::Expected
    } else {
        AnomalyState::Anomalous
    };

    let score = if state == AnomalyState::Expected || total == 0 {
        0
    } else {
        let actual_share =
            u64::from(actual_count).saturating_mul(u64::from(SCALE)) / u64::from(total);

        SCALE.saturating_sub(actual_share.min(u64::from(SCALE)) as u32)
    };

    AnomalyRecord {
        semantic_id: observation.semantic_id,
        source_state: observation.previous_state,
        actual_state: observation.new_state,
        expected_state: expected.next_state,
        context: observation.context,
        score_fixed: score,
        baseline: expected.count,
        state,
        provenance: observation.provenance,
        origin_object_id: observation.object_id,
    }
}

#[inline]
fn update_transition(engine: &mut EngineState, observation: &SemanticDelta) -> TransitionStats {
    if let Some(slot) = find_transition_slot_unbudgeted(
        engine,
        observation.object_id,
        observation.semantic_id,
        observation.previous_state,
        observation.new_state,
        observation.context,
    ) {
        let mut entry = engine.transitions[slot];

        entry.count = entry.count.saturating_add(1);
        entry.last_seen = observation.timestamp;
        entry.provenance = observation.provenance;

        engine.transitions[slot] = entry;
        return entry;
    }

    let entry = TransitionStats {
        object_id: observation.object_id,
        semantic_id: observation.semantic_id,
        previous_state: observation.previous_state,
        next_state: observation.new_state,
        context: observation.context,
        count: 1,
        prediction_attempts: 0,
        prediction_hits: 0,
        prediction_misses: 0,
        last_seen: observation.timestamp,
        provenance: observation.provenance,
    };

    let slot = insert_transition_slot(engine, entry);
    engine.transitions[slot]
}

#[inline]
fn update_state_counts(
    engine: &mut EngineState,
    observation: &SemanticDelta,
    report: &mut SubconsciousReport,
) -> bool {
    let object_id = observation.object_id;
    let pairs = [
        (
            observation.semantic_id,
            observation.previous_state,
            observation.context,
        ),
        (
            observation.semantic_id,
            observation.new_state,
            observation.context,
        ),
    ];

    let mut index = 0usize;

    while index < pairs.len() {
        let (semantic_id, state, context) = pairs[index];

        let slot = find_state_slot(engine, object_id, semantic_id, state, context, report);

        if report.stopped_by_budget {
            return false;
        }

        if let Some(slot) = slot {
            let mut entry = engine.state_counts[slot];
            entry.count = entry.count.saturating_add(1);
            engine.state_counts[slot] = entry;
        } else {
            let entry = StateCount {
                object_id,
                semantic_id,
                state,
                context,
                count: 1,
            };

            insert_state_slot(engine, entry);
        }

        index += 1;
    }

    true
}

#[inline]
fn update_pattern(
    engine: &mut EngineState,
    observation: &SemanticDelta,
    report: &mut SubconsciousReport,
) -> Option<PatternRecord> {
    let slot = find_pattern_slot(
        engine,
        observation.object_id,
        observation.semantic_id,
        observation.previous_state,
        observation.new_state,
        observation.context,
        report,
    );

    if report.stopped_by_budget {
        return None;
    }

    if let Some(slot) = slot {
        let mut pattern = engine.patterns[slot];

        pattern.support = pattern.support.saturating_add(1);
        pattern.recurrence = pattern.recurrence.saturating_add(1);
        pattern.last_seen = observation.timestamp;
        pattern.provenance = observation.provenance;

        engine.patterns[slot] = pattern;
        return Some(pattern);
    }

    let pattern = PatternRecord {
        object_id: observation.object_id,
        semantic_id: observation.semantic_id,
        previous_state: observation.previous_state,
        next_state: observation.new_state,
        context: observation.context,
        support: 1,
        first_seen: observation.timestamp,
        last_seen: observation.timestamp,
        recurrence: 1,
        provenance: observation.provenance,
    };

    let slot = insert_pattern_slot(engine, pattern);
    Some(engine.patterns[slot])
}

#[inline]
fn emit_prediction_if_eligible(
    engine: &mut EngineState,
    observation: &SemanticDelta,
    report: &mut SubconsciousReport,
) -> bool {
    let Some(dominant) = dominant_transition(
        engine,
        observation.object_id,
        observation.semantic_id,
        observation.previous_state,
        observation.context,
    ) else {
        return true;
    };

    if dominant.count < MIN_EVIDENCE {
        return true;
    }

    let total = total_outgoing_support(
        engine,
        observation.object_id,
        dominant.semantic_id,
        dominant.previous_state,
        dominant.context,
    );

    if total == 0 {
        return true;
    }

    let confidence = confidence_fixed(dominant.prediction_hits, dominant.prediction_attempts);

    let prediction = PredictionRecord {
        semantic_id: dominant.semantic_id,
        source_state: dominant.previous_state,
        target_state: dominant.next_state,
        context: dominant.context,
        support: dominant.count,
        total_outgoing: total,
        confidence_fixed: confidence,
        uncertainty_fixed: uncertainty_fixed(confidence),
        created_at: engine.observation_counter,
        outcome: PredictionOutcome::Unresolved,
        error: 0,
        provenance: dominant.provenance,
        origin_object_id: observation.object_id,
    };

    let existing_slot = find_prediction_slot(
        engine,
        prediction.origin_object_id,
        prediction.semantic_id,
        prediction.source_state,
        prediction.context,
        report,
    );

    if report.stopped_by_budget {
        return false;
    }

    if let Some(slot) = existing_slot {
        let existing = engine.predictions[slot];

        // Never replace an unresolved prediction. It is the active forecast
        // that the next matching observation must evaluate.
        if existing.outcome == PredictionOutcome::Unresolved {
            report.latest_prediction = existing;
            return true;
        }

        // A resolved prediction remains historical evidence until the slot is
        // reused for the next forecast.
        engine.predictions[slot] = prediction;
    } else {
        insert_prediction_slot(engine, prediction);
    }

    report.prediction_count = report.prediction_count.saturating_add(1);
    report.latest_prediction = prediction;

    true
}

#[inline]
fn finalize_report(
    engine: &EngineState,
    report: &mut SubconsciousReport,
    completed_normally: bool,
) {
    report.transition_count = engine.transition_used;
    report.state_count = engine.state_used;
    report.pattern_count = engine.pattern_used;

    report.raw_units = u32::try_from(report.valid_observations).unwrap_or(u32::MAX);

    // The summary is deliberately based on learned pattern cardinality.
    // It is a measurable bounded compression proxy, not a claim of
    // information-theoretic compression.
    report.summary_units = u32::try_from(
        engine
            .transition_used
            .saturating_add(engine.pattern_used)
            .saturating_add(engine.prediction_used),
    )
    .unwrap_or(u32::MAX);

    if report.summary_units == 0 {
        report.compression_x100 = 0;
    } else {
        report.compression_x100 = report
            .raw_units
            .saturating_mul(100)
            .saturating_div(report.summary_units);
    }

    report.provisional = !completed_normally;
}

/// Process a bounded batch of semantic deltas.
///
/// The caller retains ownership of the input. This API does not consume or
/// mutate the canonical Delta queue.
///
/// For canonical queue integration, use `process_pending()` below.
#[inline]

/// Copy the currently retained provisional candidates into a caller-owned
/// fixed-capacity buffer.
///
/// This is a read-only observation boundary. The caller receives copies of
/// CandidateRecord values and never receives access to the internal engine.
///
/// Returns the number of candidate records copied.
///
/// The operation is bounded by `MAX_CANDIDATES` and the capacity supplied by
/// the caller.
#[inline]
pub fn latest_candidates(output: &mut [CandidateRecord]) -> usize {
    let engine = ENGINE.lock();

    let limit = if output.len() < engine.candidate_used {
        output.len()
    } else {
        engine.candidate_used
    };

    let mut index = 0usize;

    while index < limit {
        output[index] = engine.candidates[index];
        index += 1;
    }

    limit
}

pub fn process(observations: &[SemanticDelta]) -> SubconsciousReport {
    let mut report = SubconsciousReport::empty();
    let mut engine = ENGINE.lock();

    let limit = observations.len().min(MAX_OBSERVATIONS);

    if observations.len() > MAX_OBSERVATIONS {
        report.overflow = true;
    }

    let mut index = 0usize;

    while index < limit {
        if !charge(&mut report, 1) {
            finalize_report(&engine, &mut report, false);
            return report;
        }

        let observation = observations[index];

        report.observations_seen = report.observations_seen.saturating_add(1);

        if !observation.is_valid() {
            report.invalid_observations = report.invalid_observations.saturating_add(1);
            index += 1;
            continue;
        }

        report.valid_observations = report.valid_observations.saturating_add(1);

        let key = DeltaEventKey::from_delta(&observation);

        let Some(already_seen) = find_seen_key(&engine, &key, &mut report) else {
            finalize_report(&engine, &mut report, false);
            return report;
        };

        if already_seen {
            report.duplicate_observations = report.duplicate_observations.saturating_add(1);
            index += 1;
            continue;
        }

        if !evaluate_pending_prediction(&mut engine, &observation, &mut report) {
            finalize_report(&engine, &mut report, false);
            return report;
        }

        if !charge(&mut report, 1) {
            finalize_report(&engine, &mut report, false);
            return report;
        }

        let anomaly = evaluate_anomaly(&engine, &observation);

        insert_anomaly_slot(&mut engine, anomaly);

        report.anomaly_count = report.anomaly_count.saturating_add(1);
        report.latest_anomaly = anomaly;

        if anomaly.state == AnomalyState::Anomalous {
            let candidate = CandidateRecord {
                kind: CandidateKind::Anomaly,
                semantic_id: anomaly.semantic_id,
                previous_state: anomaly.source_state,
                observed_state: anomaly.actual_state,
                predicted_state: anomaly.expected_state,
                context: anomaly.context,
                score_fixed: anomaly.score_fixed,
                uncertainty_fixed: uncertainty_fixed(SCALE.saturating_sub(anomaly.score_fixed)),
                support: anomaly.baseline,
                timestamp: observation.timestamp,
                provenance: anomaly.provenance,
                origin_object_id: anomaly.origin_object_id,
            };

            insert_candidate_slot(&mut engine, candidate);
            report.candidate_count = report.candidate_count.saturating_add(1);
            report.latest_candidate = candidate;
        }

        if !charge(&mut report, 1) {
            finalize_report(&engine, &mut report, false);
            return report;
        }

        engine.observation_counter = engine.observation_counter.saturating_add(1);

        if !charge(&mut report, 1) {
            finalize_report(&engine, &mut report, false);
            return report;
        }

        let updated_transition = update_transition(&mut engine, &observation);

        if !charge(&mut report, 1) {
            finalize_report(&engine, &mut report, false);
            return report;
        }

        if !update_state_counts(&mut engine, &observation, &mut report) {
            finalize_report(&engine, &mut report, false);
            return report;
        }

        if !charge(&mut report, 1) {
            finalize_report(&engine, &mut report, false);
            return report;
        }

        let pattern = update_pattern(&mut engine, &observation, &mut report);

        if report.stopped_by_budget {
            finalize_report(&engine, &mut report, false);
            return report;
        }

        if let Some(pattern) = pattern {
            if pattern.support >= MIN_PATTERN_EVIDENCE {
                let confidence =
                    confidence_fixed(pattern.support.min(SCALE), pattern.support.max(1));

                let candidate = CandidateRecord {
                    kind: if observation.semantic_id == super::semantic::ACTIVE
                        || observation.semantic_id == super::semantic::AVAILABLE
                    {
                        CandidateKind::ResourceTransition
                    } else {
                        CandidateKind::RepeatedTransition
                    },
                    semantic_id: pattern.semantic_id,
                    previous_state: pattern.previous_state,
                    observed_state: pattern.next_state,
                    predicted_state: pattern.next_state,
                    context: pattern.context,
                    score_fixed: confidence,
                    uncertainty_fixed: uncertainty_fixed(confidence),
                    support: pattern.support,
                    timestamp: observation.timestamp,
                    provenance: pattern.provenance,
                    origin_object_id: observation.object_id,
                };

                insert_candidate_slot(&mut engine, candidate);
                report.candidate_count = report.candidate_count.saturating_add(1);
                report.latest_candidate = candidate;
            }
        }

        if !charge(&mut report, 1) {
            finalize_report(&engine, &mut report, false);
            return report;
        }

        let evaluated_prediction = report.latest_prediction;
        let evaluated_prediction_outcome = evaluated_prediction.outcome;
        let evaluated_candidate = report.latest_candidate;

        if !emit_prediction_if_eligible(&mut engine, &observation, &mut report) {
            finalize_report(&engine, &mut report, false);
            return report;
        }

        // The report for this observation must describe the prediction that
        // was actually evaluated, not the next unresolved forecast installed
        // for future observations. Likewise, an anomaly candidate remains
        // the latest candidate when the current event is anomalous.
        if evaluated_prediction_outcome != PredictionOutcome::Unresolved {
            report.latest_prediction = evaluated_prediction;

            if evaluated_candidate.kind == CandidateKind::Anomaly {
                report.latest_candidate = evaluated_candidate;
            }
        }

        if !charge(&mut report, 1) {
            finalize_report(&engine, &mut report, false);
            return report;
        }

        let total = total_outgoing_support(
            &engine,
            observation.object_id,
            observation.semantic_id,
            observation.previous_state,
            observation.context,
        );

        report.latest_surprisal_bits = information_surprisal_bits(updated_transition.count, total);

        report.information_available = true;

        insert_seen_key(&mut engine, key);

        index += 1;
    }

    if index < limit {
        report.overflow = true;
    }

    if !charge(&mut report, 1) {
        finalize_report(&engine, &mut report, false);
        return report;
    }

    finalize_report(&engine, &mut report, true);
    report
}

/// Process canonical SemanticDelta observations directly from the Delta queue.
///
/// This is the actual background-cognition boundary:
///
///     canonical Delta queue
///             ↓
///     bounded Subconscious
///             ↓
///     derived candidates
///
/// The queue is consumed only through its existing FIFO `pop()` API.
/// No Delta mutation is performed by the Subconscious itself.
#[inline]
pub fn process_pending() -> SubconsciousReport {
    let mut report = SubconsciousReport::empty();
    let mut observations = [SemanticDelta {
        object_id: 0,
        semantic_id: 0,
        previous_state: 0,
        new_state: 0,
        context: 0,
        timestamp: 0,
        provenance: RelationProvenance {
            origin: 0,
            source: ProvenanceSource::Local,
            revision: 0,
        },
    }; MAX_OBSERVATIONS];

    let mut count = 0usize;

    while count < MAX_OBSERVATIONS {
        if !charge(&mut report, 1) {
            report.provisional = true;
            return report;
        }

        let Some(delta) = super::delta::pop() else {
            break;
        };

        observations[count] = delta;
        count += 1;
    }

    if count == 0 {
        return report;
    }

    let mut processed = process(&observations[..count]);

    processed.work_used = processed.work_used.saturating_add(report.work_used);

    if processed.work_used > MAX_WORK {
        processed.work_used = MAX_WORK;
        processed.overflow = true;
        processed.stopped_by_budget = true;
        processed.provisional = true;
    }

    processed
}

#[inline]
fn make_delta(
    object_id: u64,
    semantic_id: SemanticId,
    previous_state: SemanticId,
    new_state: SemanticId,
    context: ContextId,
    timestamp: u64,
    revision: u64,
) -> SemanticDelta {
    SemanticDelta {
        object_id,
        semantic_id,
        previous_state,
        new_state,
        context,
        timestamp,
        provenance: RelationProvenance {
            origin: object_id,
            source: ProvenanceSource::Local,
            revision,
        },
    }
}

/// Deterministic REL-09 integration self-test.
///
/// The test verifies:
/// - integer confidence
/// - uncertainty complement
/// - bounded surprisal
/// - no single-event fabricated prediction
/// - repeated-event learning
/// - transition pattern detection
/// - prediction generation
/// - prediction hit
/// - prediction miss
/// - anomaly detection
/// - candidate generation
/// - bounded processing
/// - duplicate suppression
/// - deterministic replay
/// - persistence of derived learning across calls
/// - canonical GNOSIS non-mutation
/// - exact FIFO Delta interaction through process_pending
#[inline]
pub fn self_test() -> bool {
    let before_delta_count = super::delta::count();
    let before_attention_count = super::attention::count();
    let before_relation_count = super::relation::count();
    let before_knowledge_count = super::knowledge::entry_count();

    clear();

    if confidence_fixed(0, 0) != 0
        || confidence_fixed(1, 0) != 0
        || confidence_fixed(1, 5) != 2000
        || confidence_fixed(2, 5) != 4000
        || confidence_fixed(5, 5) != SCALE
        || confidence_fixed(u32::MAX, 1) != SCALE
        || uncertainty_fixed(0) != SCALE
        || uncertainty_fixed(SCALE) != 0
        || uncertainty_fixed(2500) != 7500
    {
        return false;
    }

    let s1 = information_surprisal_bits(5, 5);
    let s2 = information_surprisal_bits(1, 5);
    let s3 = information_surprisal_bits(1, 1000);

    if s1 > s2 || s2 > s3 || s3 > MAX_SURPRISAL_BITS {
        return false;
    }

    // One observation cannot establish a learned prediction.
    let single = [make_delta(
        10,
        super::semantic::CONNECTED_TO,
        super::semantic::READY,
        super::semantic::ACTIVE,
        1,
        1,
        1,
    )];

    let single_report = process(&single);

    if single_report.prediction_count != 0 {
        return false;
    }

    if single_report.latest_anomaly.state != AnomalyState::Unknown {
        return false;
    }

    // Three repeated observations create defensible transition evidence.
    let train = [
        make_delta(
            20,
            super::semantic::CAUSES,
            super::semantic::PENDING,
            super::semantic::COMPLETE,
            9,
            10,
            10,
        ),
        make_delta(
            20,
            super::semantic::CAUSES,
            super::semantic::PENDING,
            super::semantic::COMPLETE,
            9,
            11,
            11,
        ),
        make_delta(
            20,
            super::semantic::CAUSES,
            super::semantic::PENDING,
            super::semantic::COMPLETE,
            9,
            12,
            12,
        ),
    ];

    let learned = process(&train);

    if learned.prediction_count == 0 {
        return false;
    }

    if learned.latest_prediction.support < MIN_EVIDENCE {
        return false;
    }

    if learned.latest_prediction.uncertainty_fixed != SCALE {
        return false;
    }

    if learned.latest_candidate.kind != CandidateKind::Prediction
        && learned.latest_candidate.kind != CandidateKind::RepeatedTransition
    {
        return false;
    }

    // Repeated transition must be represented as a pattern.
    let engine = ENGINE.lock();

    let pattern_slot = match find_pattern_slot_unbudgeted(
        &engine,
        20,
        super::semantic::CAUSES,
        super::semantic::PENDING,
        super::semantic::COMPLETE,
        9,
    ) {
        Some(slot) => slot,
        None => {
            drop(engine);
            return false;
        }
    };

    if engine.patterns[pattern_slot].support < MIN_PATTERN_EVIDENCE {
        drop(engine);
        return false;
    }

    drop(engine);

    // Matching observation must close prediction as a hit.
    let hit = [make_delta(
        20,
        super::semantic::CAUSES,
        super::semantic::PENDING,
        super::semantic::COMPLETE,
        9,
        13,
        13,
    )];

    let hit_report = process(&hit);

    if hit_report.hit_count == 0 {
        return false;
    }

    if hit_report.latest_anomaly.state != AnomalyState::Expected {
        return false;
    }

    if hit_report.latest_anomaly.score_fixed != 0 {
        return false;
    }

    // Divergent transition must close the prediction as a miss and become
    // statistically anomalous.
    let miss = [make_delta(
        20,
        super::semantic::CAUSES,
        super::semantic::PENDING,
        super::semantic::FAILED,
        9,
        14,
        14,
    )];

    let miss_report = process(&miss);

    if miss_report.miss_count == 0 {
        return false;
    }

    if miss_report.latest_prediction.error == 0 {
        return false;
    }

    if miss_report.latest_prediction.outcome != PredictionOutcome::Miss {
        return false;
    }

    if miss_report.latest_anomaly.state != AnomalyState::Anomalous {
        return false;
    }

    if miss_report.latest_anomaly.score_fixed == 0 {
        return false;
    }

    if miss_report.latest_candidate.kind != CandidateKind::Anomaly {
        return false;
    }

    // Learning persists and confidence reflects observed prediction history.
    let engine = ENGINE.lock();

    let stats = match find_transition_slot_unbudgeted(
        &engine,
        20,
        super::semantic::CAUSES,
        super::semantic::PENDING,
        super::semantic::COMPLETE,
        9,
    ) {
        Some(slot) => engine.transitions[slot],
        None => {
            drop(engine);
            return false;
        }
    };

    if stats.prediction_attempts == 0 || stats.prediction_hits == 0 || stats.prediction_misses == 0
    {
        drop(engine);
        return false;
    }

    let confidence = confidence_fixed(stats.prediction_hits, stats.prediction_attempts);

    if confidence >= SCALE {
        drop(engine);
        return false;
    }

    if uncertainty_fixed(confidence) == 0 {
        drop(engine);
        return false;
    }

    drop(engine);

    // Object isolation: opposing distributions for two objects must never
    // contaminate one another. Both objects share semantic/context/source state,
    // but their learned target distributions are intentionally different.
    clear();

    let object_a = [
        make_delta(
            400,
            super::semantic::CAUSES,
            super::semantic::PENDING,
            super::semantic::COMPLETE,
            21,
            40,
            40,
        ),
        make_delta(
            400,
            super::semantic::CAUSES,
            super::semantic::PENDING,
            super::semantic::COMPLETE,
            21,
            41,
            41,
        ),
        make_delta(
            400,
            super::semantic::CAUSES,
            super::semantic::PENDING,
            super::semantic::COMPLETE,
            21,
            42,
            42,
        ),
    ];

    let object_b = [
        make_delta(
            401,
            super::semantic::CAUSES,
            super::semantic::PENDING,
            super::semantic::FAILED,
            21,
            50,
            50,
        ),
        make_delta(
            401,
            super::semantic::CAUSES,
            super::semantic::PENDING,
            super::semantic::FAILED,
            21,
            51,
            51,
        ),
        make_delta(
            401,
            super::semantic::CAUSES,
            super::semantic::PENDING,
            super::semantic::FAILED,
            21,
            52,
            52,
        ),
    ];

    process(&object_a);
    process(&object_b);

    let engine = ENGINE.lock();

    let a_complete = find_transition_slot_unbudgeted(
        &engine,
        400,
        super::semantic::CAUSES,
        super::semantic::PENDING,
        super::semantic::COMPLETE,
        21,
    );
    let a_failed = find_transition_slot_unbudgeted(
        &engine,
        400,
        super::semantic::CAUSES,
        super::semantic::PENDING,
        super::semantic::FAILED,
        21,
    );
    let b_complete = find_transition_slot_unbudgeted(
        &engine,
        401,
        super::semantic::CAUSES,
        super::semantic::PENDING,
        super::semantic::COMPLETE,
        21,
    );
    let b_failed = find_transition_slot_unbudgeted(
        &engine,
        401,
        super::semantic::CAUSES,
        super::semantic::PENDING,
        super::semantic::FAILED,
        21,
    );

    if a_complete.is_none()
        || a_failed.is_some()
        || b_complete.is_some()
        || b_failed.is_none()
        || engine.transitions[a_complete.unwrap()].count != 3
        || engine.transitions[b_failed.unwrap()].count != 3
    {
        drop(engine);
        return false;
    }

    let a_dominant = dominant_transition(
        &engine,
        400,
        super::semantic::CAUSES,
        super::semantic::PENDING,
        21,
    );
    let b_dominant = dominant_transition(
        &engine,
        401,
        super::semantic::CAUSES,
        super::semantic::PENDING,
        21,
    );

    if a_dominant.map(|entry| entry.next_state) != Some(super::semantic::COMPLETE)
        || b_dominant.map(|entry| entry.next_state) != Some(super::semantic::FAILED)
    {
        drop(engine);
        return false;
    }

    drop(engine);

    let a_prediction = process(&[make_delta(
        400,
        super::semantic::CAUSES,
        super::semantic::PENDING,
        super::semantic::COMPLETE,
        21,
        60,
        60,
    )]);

    if a_prediction.latest_prediction.origin_object_id != 400
        || a_prediction.latest_prediction.target_state != super::semantic::COMPLETE
        || a_prediction.latest_prediction.outcome != PredictionOutcome::Hit
    {
        return false;
    }

    let b_prediction = process(&[make_delta(
        401,
        super::semantic::CAUSES,
        super::semantic::PENDING,
        super::semantic::FAILED,
        21,
        61,
        61,
    )]);

    if b_prediction.latest_prediction.origin_object_id != 401
        || b_prediction.latest_prediction.target_state != super::semantic::FAILED
        || b_prediction.latest_prediction.outcome != PredictionOutcome::Hit
    {
        return false;
    }

    // Duplicate input must not learn twice.
    clear();

    let duplicate = make_delta(
        300,
        super::semantic::CAUSES,
        super::semantic::PENDING,
        super::semantic::COMPLETE,
        15,
        30,
        30,
    );

    let first = process(&[duplicate]);
    let second = process(&[duplicate]);

    if first.valid_observations != 1 || second.duplicate_observations != 1 {
        return false;
    }

    let engine = ENGINE.lock();

    let duplicate_slot = match find_transition_slot_unbudgeted(
        &engine,
        300,
        super::semantic::CAUSES,
        super::semantic::PENDING,
        super::semantic::COMPLETE,
        15,
    ) {
        Some(slot) => slot,
        None => {
            drop(engine);
            return false;
        }
    };

    if engine.transitions[duplicate_slot].count != 1 {
        drop(engine);
        return false;
    }

    drop(engine);

    // Bounded processing must never exceed the declared work budget.
    clear();

    let mut flood = [make_delta(
        1000,
        super::semantic::CONNECTED_TO,
        super::semantic::READY,
        super::semantic::ACTIVE,
        77,
        1000,
        1000,
    ); MAX_OBSERVATIONS];

    let mut index = 0usize;

    while index < MAX_OBSERVATIONS {
        flood[index] = make_delta(
            1000 + index as u64,
            super::semantic::CONNECTED_TO,
            super::semantic::READY,
            super::semantic::ACTIVE,
            77,
            1000 + index as u64,
            1000 + index as u64,
        );

        index += 1;
    }

    let flood_report = process(&flood);

    if flood_report.work_used > MAX_WORK {
        return false;
    }

    // Deterministic replay.
    clear();

    let deterministic_sequence = [
        make_delta(
            700,
            super::semantic::CAUSES,
            super::semantic::PENDING,
            super::semantic::COMPLETE,
            12,
            1,
            1,
        ),
        make_delta(
            701,
            super::semantic::CAUSES,
            super::semantic::PENDING,
            super::semantic::COMPLETE,
            12,
            2,
            2,
        ),
        make_delta(
            702,
            super::semantic::CAUSES,
            super::semantic::PENDING,
            super::semantic::COMPLETE,
            12,
            3,
            3,
        ),
        make_delta(
            703,
            super::semantic::CAUSES,
            super::semantic::PENDING,
            super::semantic::FAILED,
            12,
            4,
            4,
        ),
    ];

    let deterministic_a = process(&deterministic_sequence);

    clear();

    let deterministic_b = process(&deterministic_sequence);

    if deterministic_a != deterministic_b {
        return false;
    }

    // Canonical Delta queue is not touched by ordinary process().
    clear();

    let before_pending_delta_count = super::delta::count();

    let queued = make_delta(
        800,
        super::semantic::CAUSES,
        super::semantic::PENDING,
        super::semantic::COMPLETE,
        13,
        50,
        50,
    );

    if super::delta::push(queued).is_err() {
        return false;
    }

    if super::delta::count() != before_pending_delta_count.saturating_add(1) {
        return false;
    }

    let pending_report = process_pending();

    if pending_report.valid_observations == 0 {
        return false;
    }

    if super::delta::count() != before_pending_delta_count {
        return false;
    }

    // Reset affects only derived Subconscious state.
    clear();

    let after_reset = process(&[]);

    if after_reset.transition_count != 0
        || after_reset.state_count != 0
        || after_reset.pattern_count != 0
        || after_reset.prediction_count != 0
        || after_reset.anomaly_count != 0
        || after_reset.candidate_count != 0
    {
        return false;
    }

    if super::delta::count() != before_pending_delta_count {
        return false;
    }

    if super::delta::count() != before_delta_count
        || super::attention::count() != before_attention_count
        || super::relation::count() != before_relation_count
        || super::knowledge::entry_count() != before_knowledge_count
    {
        return false;
    }

    clear();
    true
}

/// Clear all derived Subconscious state.
///
/// This does not mutate any canonical GNOSIS subsystem.
#[inline]
pub fn clear() {
    *ENGINE.lock() = EngineState::empty();
}
