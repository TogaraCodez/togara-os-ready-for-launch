//! GNOSIS canonical KOBJ transition transaction boundary.
//!
//! This module owns the lifecycle:
//!
//!     prepare canonical transition
//!         -> reserve Delta capacity
//!         -> commit canonical state
//!         -> commit reserved SemanticDelta
//!         -> record publication
//!
//! The reservation is the critical atomicity primitive. Once a valid
//! transition has been prepared and a Delta slot reserved, ordinary Delta
//! publication cannot fail because the queue became full or busy.
//!
//! Canonical state remains owned by `objects`.
//! Derived Delta state remains owned by `delta`.
//! This module coordinates the two without exposing an independent
//! canonical-mutation path.

use super::delta::{self, DeltaError, DeltaReservation, SemanticDelta};
use super::objects::{self, StateTransition};
use super::relation::{ProvenanceSource, RelationProvenance};

pub const MAX_PUBLISHED: usize = delta::MAX_DELTAS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublishError {
    Invalid,
    Full,
    Busy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublishResult {
    Published,
    AlreadyPublished,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionResult {
    Published(StateTransition),
    AlreadyPublished(StateTransition),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PublicationRecord {
    transition: StateTransition,
    provenance: RelationProvenance,
}

impl PublicationRecord {
    const fn empty() -> Self {
        Self {
            transition: StateTransition {
                object_id: 0,
                semantic_id: 0,
                previous_state: 0,
                new_state: 0,
                context: 0,
                timestamp: 0,
                version: 0,
            },
            provenance: RelationProvenance {
                origin: 0,
                source: ProvenanceSource::Local,
                revision: 0,
            },
        }
    }
}

#[derive(Clone, Copy)]
struct JournalState {
    records: [PublicationRecord; MAX_PUBLISHED],
    count: usize,
    reserved: bool,
}

impl JournalState {
    const fn empty() -> Self {
        Self {
            records: [PublicationRecord::empty(); MAX_PUBLISHED],
            count: 0,
            reserved: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct JournalReservation;

static JOURNAL: spin::Mutex<JournalState> = spin::Mutex::new(JournalState::empty());

#[inline]
fn transition_valid(transition: &StateTransition) -> bool {
    transition.object_id != 0
        && super::semantic::is_valid(transition.semantic_id)
        && super::semantic::is_valid(transition.previous_state)
        && super::semantic::is_valid(transition.new_state)
        && transition.previous_state != transition.new_state
        && transition.version != 0
}

#[inline]
fn provenance_valid(provenance: &RelationProvenance) -> bool {
    provenance.revision != 0
        && matches!(
            provenance.source,
            ProvenanceSource::Local | ProvenanceSource::Remote | ProvenanceSource::Recovered
        )
}

#[inline]
fn same_transition(lhs: &StateTransition, rhs: &StateTransition) -> bool {
    lhs.object_id == rhs.object_id
        && lhs.semantic_id == rhs.semantic_id
        && lhs.previous_state == rhs.previous_state
        && lhs.new_state == rhs.new_state
        && lhs.context == rhs.context
        && lhs.timestamp == rhs.timestamp
        && lhs.version == rhs.version
}

#[inline]
fn find_record(transition: &StateTransition, provenance: &RelationProvenance) -> Option<usize> {
    let journal = JOURNAL.lock();

    let mut index = 0usize;

    while index < journal.count {
        let record = journal.records[index];

        if same_transition(&record.transition, transition) && record.provenance == *provenance {
            return Some(index);
        }

        index += 1;
    }

    None
}

#[inline]
pub fn count() -> usize {
    JOURNAL.lock().count
}

#[inline]
pub const fn capacity() -> usize {
    MAX_PUBLISHED
}

#[inline]
pub fn clear() {
    let mut journal = JOURNAL.lock();
    *journal = JournalState::empty();
}

#[inline]
fn reserve_journal() -> Result<JournalReservation, PublishError> {
    let mut journal = JOURNAL.lock();

    if journal.reserved {
        return Err(PublishError::Busy);
    }

    if journal.count >= MAX_PUBLISHED {
        return Err(PublishError::Full);
    }

    journal.reserved = true;

    Ok(JournalReservation)
}

#[inline]
fn cancel_journal_reservation(_reservation: JournalReservation) -> bool {
    let mut journal = JOURNAL.lock();

    if !journal.reserved {
        return false;
    }

    journal.reserved = false;
    true
}

#[inline]
fn commit_journal_reservation(
    _reservation: JournalReservation,
    transition: StateTransition,
    provenance: RelationProvenance,
) -> Result<(), PublishError> {
    let mut journal = JOURNAL.lock();

    if !journal.reserved {
        return Err(PublishError::Busy);
    }

    if journal.count >= MAX_PUBLISHED {
        journal.reserved = false;
        return Err(PublishError::Full);
    }

    let index = journal.count;

    journal.records[index] = PublicationRecord {
        transition,
        provenance,
    };

    journal.count = index + 1;
    journal.reserved = false;

    Ok(())
}

/// Publish an already-constructed transition.
///
/// This low-level function is retained for publication of externally
/// constructed immutable transition records. Canonical mutation must not be
/// performed separately and then assumed to be atomic with this call.
///
/// New canonical state changes should use `transition_and_publish()`.
#[inline]
pub fn publish(
    transition: StateTransition,
    provenance: RelationProvenance,
) -> Result<PublishResult, PublishError> {
    if !transition_valid(&transition) || !provenance_valid(&provenance) {
        return Err(PublishError::Invalid);
    }

    if find_record(&transition, &provenance).is_some() {
        return Ok(PublishResult::AlreadyPublished);
    }

    let journal_reservation = reserve_journal()?;

    let delta_reservation = match delta::reserve() {
        Ok(value) => value,

        Err(DeltaError::Full) => {
            cancel_journal_reservation(journal_reservation);
            return Err(PublishError::Full);
        }

        Err(DeltaError::Busy) => {
            cancel_journal_reservation(journal_reservation);
            return Err(PublishError::Busy);
        }

        Err(DeltaError::Invalid) => {
            cancel_journal_reservation(journal_reservation);
            return Err(PublishError::Invalid);
        }
    };

    let result = publish_reserved(
        delta_reservation,
        journal_reservation,
        transition,
        provenance,
    );

    if result.is_err() {
        delta::cancel_reservation(delta_reservation);
        cancel_journal_reservation(journal_reservation);
    }

    result
}

/// Atomically coordinate canonical KOBJ mutation with SemanticDelta
/// publication.
///
/// The canonical object is not mutated until the Delta queue has reserved
/// capacity. The reservation is then consumed only after the canonical
/// transition commits successfully.
#[inline]
pub fn transition_and_publish(
    object_id: super::semantic::ObjectId,
    new_state: super::semantic::SemanticId,
    timestamp: u64,
    provenance: RelationProvenance,
) -> Result<TransitionResult, PublishError> {
    if !provenance_valid(&provenance) {
        return Err(PublishError::Invalid);
    }

    let prepared = match objects::prepare_transition(object_id, new_state, timestamp) {
        Some(value) => value,
        None => return Err(PublishError::Invalid),
    };

    if find_record(&prepared, &provenance).is_some() {
        return Ok(TransitionResult::AlreadyPublished(prepared));
    }

    // Reserve both derived publication slots before mutating canonical state.
    // This makes journal exhaustion just as atomic as Delta exhaustion.
    let journal_reservation = reserve_journal()?;

    let delta_reservation = match delta::reserve() {
        Ok(value) => value,

        Err(DeltaError::Full) => {
            cancel_journal_reservation(journal_reservation);
            return Err(PublishError::Full);
        }

        Err(DeltaError::Busy) => {
            cancel_journal_reservation(journal_reservation);
            return Err(PublishError::Busy);
        }

        Err(DeltaError::Invalid) => {
            cancel_journal_reservation(journal_reservation);
            return Err(PublishError::Invalid);
        }
    };

    if !objects::commit_transition(prepared) {
        delta::cancel_reservation(delta_reservation);
        cancel_journal_reservation(journal_reservation);
        return Err(PublishError::Busy);
    }

    let delta = SemanticDelta {
        object_id: prepared.object_id,
        semantic_id: prepared.semantic_id,
        previous_state: prepared.previous_state,
        new_state: prepared.new_state,
        context: prepared.context,
        timestamp: prepared.timestamp,
        provenance,
    };

    if delta::commit_reserved(delta_reservation, delta).is_err() {
        // A valid Delta reservation plus a valid Delta cannot normally fail.
        // Keep the failure explicit; canonical state has already committed, so
        // this is treated as an invariant violation rather than fabricated success.
        cancel_journal_reservation(journal_reservation);
        return Err(PublishError::Busy);
    }

    if commit_journal_reservation(journal_reservation, prepared, provenance).is_err() {
        return Err(PublishError::Busy);
    }

    Ok(TransitionResult::Published(prepared))
}

#[inline]
fn publish_reserved(
    delta_reservation: DeltaReservation,
    journal_reservation: JournalReservation,
    transition: StateTransition,
    provenance: RelationProvenance,
) -> Result<PublishResult, PublishError> {
    let delta = SemanticDelta {
        object_id: transition.object_id,
        semantic_id: transition.semantic_id,
        previous_state: transition.previous_state,
        new_state: transition.new_state,
        context: transition.context,
        timestamp: transition.timestamp,
        provenance,
    };

    match delta::commit_reserved(delta_reservation, delta) {
        Ok(_) => {
            commit_journal_reservation(journal_reservation, transition, provenance)?;

            Ok(PublishResult::Published)
        }

        Err(DeltaError::Invalid) => Err(PublishError::Invalid),
        Err(DeltaError::Full) => Err(PublishError::Full),
        Err(DeltaError::Busy) => Err(PublishError::Busy),
    }
}

/// Convenience boundary: publish a transition with explicit local provenance.
#[inline]
pub fn publish_local(
    transition: StateTransition,
    origin: u64,
    revision: u64,
) -> Result<PublishResult, PublishError> {
    publish(
        transition,
        RelationProvenance {
            origin,
            source: ProvenanceSource::Local,
            revision,
        },
    )
}

#[inline]
pub fn self_test() -> bool {
    clear();
    delta::clear();
    objects::clear();

    let object = objects::KnowledgeObject {
        id: 7,
        semantic_type: super::semantic::STATE_OBJECT,
        state: super::semantic::READY,
        context: 77,
        flags: objects::FLAG_ACTIVE,
        version: objects::CURRENT_VERSION,
    };

    if !objects::insert(object) {
        clear();
        delta::clear();
        objects::clear();
        return false;
    }

    let provenance = RelationProvenance {
        origin: 99,
        source: ProvenanceSource::Local,
        revision: 1,
    };

    let result = transition_and_publish(object.id, super::semantic::ACTIVE, 1234, provenance);

    let transition = match result {
        Ok(TransitionResult::Published(value)) => value,

        _ => {
            clear();
            delta::clear();
            objects::clear();
            return false;
        }
    };

    if transition.previous_state != super::semantic::READY
        || transition.new_state != super::semantic::ACTIVE
        || transition.version != object.version + 1
        || count() != 1
        || delta::count() != 1
    {
        clear();
        delta::clear();
        objects::clear();
        return false;
    }

    let mut observed = None;

    objects::with_object(object.id, |value| {
        observed = Some(*value);
    });

    if observed.map(|value| value.state) != Some(super::semantic::ACTIVE)
        || observed.map(|value| value.version) != Some(transition.version)
    {
        clear();
        delta::clear();
        objects::clear();
        return false;
    }

    let duplicate = transition_and_publish(object.id, super::semantic::ACTIVE, 1234, provenance);

    if duplicate.is_ok() {
        clear();
        delta::clear();
        objects::clear();
        return false;
    }

    let invalid_provenance = RelationProvenance {
        revision: 0,
        ..provenance
    };

    if transition_and_publish(object.id, super::semantic::READY, 1235, invalid_provenance)
        != Err(PublishError::Invalid)
    {
        clear();
        delta::clear();
        objects::clear();
        return false;
    }

    let invalid = StateTransition {
        previous_state: transition.new_state,
        new_state: transition.new_state,
        ..transition
    };

    if publish(invalid, provenance) != Err(PublishError::Invalid) {
        clear();
        delta::clear();
        objects::clear();
        return false;
    }

    let mut observed_delta = None;

    delta::for_each(|value| {
        observed_delta = Some(*value);
    });

    let expected_delta = SemanticDelta {
        object_id: transition.object_id,
        semantic_id: transition.semantic_id,
        previous_state: transition.previous_state,
        new_state: transition.new_state,
        context: transition.context,
        timestamp: transition.timestamp,
        provenance,
    };

    if observed_delta != Some(expected_delta) {
        clear();
        delta::clear();
        objects::clear();
        return false;
    }

    // Journal-full atomicity: exhausting the publication journal must block
    // canonical mutation before commit, even when Delta still has capacity.
    clear();
    delta::clear();
    objects::clear();

    let fill_provenance = RelationProvenance {
        origin: 700,
        source: ProvenanceSource::Local,
        revision: 1,
    };

    let mut fill_index = 0usize;

    while fill_index < MAX_PUBLISHED {
        let record = StateTransition {
            object_id: 10_000 + fill_index as u64,
            semantic_id: super::semantic::ACTIVE,
            previous_state: super::semantic::READY,
            new_state: super::semantic::ACTIVE,
            context: fill_index as u64,
            timestamp: 2_000 + fill_index as u64,
            version: 1,
        };

        if publish(record, fill_provenance) != Ok(PublishResult::Published) {
            clear();
            delta::clear();
            objects::clear();
            return false;
        }

        // Consume Delta so the journal, rather than Delta capacity, is the
        // limiting resource for the next transition.
        if delta::pop().is_none() {
            clear();
            delta::clear();
            objects::clear();
            return false;
        }

        fill_index += 1;
    }

    let blocked_object = objects::KnowledgeObject {
        id: 77,
        semantic_type: super::semantic::STATE_OBJECT,
        state: super::semantic::READY,
        context: 77,
        flags: objects::FLAG_ACTIVE,
        version: objects::CURRENT_VERSION,
    };

    if !objects::insert(blocked_object) {
        clear();
        delta::clear();
        objects::clear();
        return false;
    }

    if transition_and_publish(
        blocked_object.id,
        super::semantic::ACTIVE,
        9_999,
        provenance,
    ) != Err(PublishError::Full)
    {
        clear();
        delta::clear();
        objects::clear();
        return false;
    }

    let mut blocked_state = None;

    objects::with_object(blocked_object.id, |value| {
        blocked_state = Some(*value);
    });

    if blocked_state.map(|value| value.state) != Some(super::semantic::READY)
        || blocked_state.map(|value| value.version) != Some(blocked_object.version)
    {
        clear();
        delta::clear();
        objects::clear();
        return false;
    }

    clear();
    delta::clear();
    objects::clear();

    count() == 0 && delta::count() == 0 && objects::count() == 0
}
