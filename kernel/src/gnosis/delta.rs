//! GNOSIS semantic delta engine.
//!
//! This module records derived semantic state transitions without mutating the
//! canonical GNOSIS registries. It is intentionally heap-free, fixed-capacity,
//! and deterministic, matching the repository's existing GNOSIS conventions.

use spin::Mutex;

use super::relation::{ProvenanceSource, RelationProvenance};
use super::semantic::{ContextId, ObjectId, SemanticId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeltaError {
    Invalid,
    Full,
    Busy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SemanticDelta {
    pub object_id: ObjectId,
    pub semantic_id: SemanticId,
    pub previous_state: SemanticId,
    pub new_state: SemanticId,
    pub context: ContextId,
    pub timestamp: u64,
    pub provenance: RelationProvenance,
}

impl SemanticDelta {
    pub const fn is_valid(&self) -> bool {
        self.object_id != 0
            && super::semantic::is_valid(self.semantic_id)
            && super::semantic::is_valid(self.previous_state)
            && super::semantic::is_valid(self.new_state)
            && self.previous_state != self.new_state
            && self.provenance.revision != 0
            && matches!(
                self.provenance.source,
                ProvenanceSource::Local | ProvenanceSource::Remote | ProvenanceSource::Recovered
            )
    }
}

pub const MAX_DELTAS: usize = 64;
pub const CAPACITY: usize = MAX_DELTAS;

#[derive(Clone, Copy)]
struct DeltaQueue {
    slots: [Option<SemanticDelta>; MAX_DELTAS],
    head: usize,
    count: usize,
    reserved: bool,
    observers: usize,
}

impl DeltaQueue {
    const fn new() -> Self {
        Self {
            slots: [None; MAX_DELTAS],
            head: 0,
            count: 0,
            reserved: false,
            observers: 0,
        }
    }

    fn clear(&mut self) -> bool {
        if self.reserved || self.observers != 0 {
            return false;
        }

        self.slots = [None; MAX_DELTAS];
        self.head = 0;
        self.count = 0;

        true
    }

    fn push(&mut self, delta: SemanticDelta) -> Result<usize, DeltaError> {
        if self.reserved || self.observers != 0 {
            return Err(DeltaError::Busy);
        }

        if self.count >= MAX_DELTAS {
            return Err(DeltaError::Full);
        }

        let index = (self.head + self.count) % MAX_DELTAS;

        self.slots[index] = Some(delta);
        self.count += 1;

        Ok(index)
    }

    fn reserve(&mut self) -> Result<DeltaReservation, DeltaError> {
        if self.reserved || self.observers != 0 {
            return Err(DeltaError::Busy);
        }

        if self.count >= MAX_DELTAS {
            return Err(DeltaError::Full);
        }

        self.reserved = true;

        Ok(DeltaReservation)
    }

    fn commit_reserved(&mut self, delta: SemanticDelta) -> Result<usize, DeltaError> {
        if !self.reserved || self.observers != 0 {
            return Err(DeltaError::Busy);
        }

        if !delta.is_valid() {
            return Err(DeltaError::Invalid);
        }

        if self.count >= MAX_DELTAS {
            return Err(DeltaError::Full);
        }

        let index = (self.head + self.count) % MAX_DELTAS;

        self.slots[index] = Some(delta);
        self.count += 1;
        self.reserved = false;

        Ok(index)
    }

    fn cancel_reservation(&mut self) -> bool {
        if !self.reserved {
            return false;
        }

        self.reserved = false;

        true
    }

    fn pop(&mut self) -> Option<SemanticDelta> {
        if self.reserved || self.observers != 0 || self.count == 0 {
            return None;
        }

        let value = self.slots[self.head];

        self.slots[self.head] = None;
        self.head = (self.head + 1) % MAX_DELTAS;
        self.count -= 1;

        value
    }
}

/// A reservation represents one exclusively owned, still-unpublished Delta
/// slot. Only one reservation may exist at a time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeltaReservation;

static DELTA_QUEUE: Mutex<DeltaQueue> = Mutex::new(DeltaQueue::new());

#[inline]
fn make_default_provenance(revision: u64) -> RelationProvenance {
    RelationProvenance {
        origin: 0,
        source: ProvenanceSource::Local,
        revision,
    }
}

#[inline]
fn current_timestamp() -> u64 {
    crate::system::state::uptime_ticks()
}

#[inline]
pub fn count() -> usize {
    DELTA_QUEUE.lock().count
}

#[inline]
pub fn head() -> usize {
    DELTA_QUEUE.lock().head
}

#[inline]
pub fn snapshot_into(out: &mut [Option<SemanticDelta>; MAX_DELTAS]) -> usize {
    let queue = DELTA_QUEUE.lock();

    let mut index = 0usize;

    while index < MAX_DELTAS {
        out[index] = queue.slots[index];
        index += 1;
    }

    queue.count
}

/// Copy the canonical Delta queue in FIFO order under one queue lock.
///
/// The returned snapshot and count originate from the same queue state.
#[inline]
pub fn snapshot_fifo_into(out: &mut [Option<SemanticDelta>; MAX_DELTAS]) -> usize {
    let queue = DELTA_QUEUE.lock();

    let count = queue.count;
    let head = queue.head;

    let mut index = 0usize;

    while index < MAX_DELTAS {
        out[index] = None;
        index += 1;
    }

    let mut offset = 0usize;

    while offset < count {
        let slot = (head + offset) % MAX_DELTAS;
        out[offset] = queue.slots[slot];
        offset += 1;
    }

    count
}

/// Observe all queued deltas in FIFO order without consuming them.
///
/// Observer state is established while the queue lock is held. Producers,
/// consumers, clearing, and reservations are therefore blocked for the entire
/// observation window.
pub fn for_each<F: FnMut(&SemanticDelta)>(mut callback: F) {
    let mut snapshot = [None; MAX_DELTAS];
    let count;

    {
        let mut queue = DELTA_QUEUE.lock();

        if queue.reserved {
            return;
        }

        count = queue.count;

        let head = queue.head;
        let mut offset = 0usize;

        while offset < count {
            let slot = (head + offset) % MAX_DELTAS;
            snapshot[offset] = queue.slots[slot];
            offset += 1;
        }

        queue.observers = queue.observers.saturating_add(1);
    }

    let mut offset = 0usize;

    while offset < count {
        if let Some(delta) = snapshot[offset] {
            callback(&delta);
        }

        offset += 1;
    }

    let mut queue = DELTA_QUEUE.lock();
    queue.observers = queue.observers.saturating_sub(1);
}

#[inline]
pub const fn capacity() -> usize {
    MAX_DELTAS
}

#[inline]
pub fn clear() {
    let _ = DELTA_QUEUE.lock().clear();
}

#[inline]
pub fn reserve() -> Result<DeltaReservation, DeltaError> {
    DELTA_QUEUE.lock().reserve()
}

#[inline]
pub fn commit_reserved(
    reservation: DeltaReservation,
    delta: SemanticDelta,
) -> Result<usize, DeltaError> {
    let _ = reservation;
    DELTA_QUEUE.lock().commit_reserved(delta)
}

#[inline]
pub fn cancel_reservation(reservation: DeltaReservation) -> bool {
    let _ = reservation;
    DELTA_QUEUE.lock().cancel_reservation()
}

#[inline]
pub fn push(delta: SemanticDelta) -> Result<usize, DeltaError> {
    if !delta.is_valid() {
        return Err(DeltaError::Invalid);
    }

    DELTA_QUEUE.lock().push(delta)
}

#[inline]
pub fn pop() -> Option<SemanticDelta> {
    DELTA_QUEUE.lock().pop()
}

#[inline]
fn sample_delta(
    object_id: ObjectId,
    semantic_id: SemanticId,
    previous_state: SemanticId,
    new_state: SemanticId,
    context: ContextId,
    revision: u64,
) -> SemanticDelta {
    SemanticDelta {
        object_id,
        semantic_id,
        previous_state,
        new_state,
        context,
        timestamp: current_timestamp(),
        provenance: make_default_provenance(revision),
    }
}

#[inline]
fn equivalent(lhs: &SemanticDelta, rhs: &SemanticDelta) -> bool {
    lhs == rhs
}

#[inline]
fn collect_fifo(out: &mut [Option<SemanticDelta>; MAX_DELTAS]) -> usize {
    let mut seen = 0usize;

    for_each(|delta| {
        if seen < MAX_DELTAS {
            out[seen] = Some(*delta);
            seen += 1;
        }
    });

    seen
}

#[inline]
pub fn self_test() -> bool {
    clear();

    // ---------------------------------------------------------------------
    // STAGE 01: clean initial state
    // ---------------------------------------------------------------------

    if count() != 0 || head() != 0 {
        return false;
    }

    // ---------------------------------------------------------------------
    // STAGE 02: valid delta construction
    // ---------------------------------------------------------------------

    let first = sample_delta(
        42,
        super::semantic::ACTIVE,
        super::semantic::READY,
        super::semantic::ACTIVE,
        9,
        1,
    );

    if !first.is_valid() {
        return false;
    }

    if push(first) != Ok(0) {
        return false;
    }

    if count() != 1 || head() != 0 {
        return false;
    }

    // ---------------------------------------------------------------------
    // STAGE 03: observation is read-only
    // ---------------------------------------------------------------------

    let before_head = head();
    let before_count = count();

    let mut observed = None;

    for_each(|delta| {
        observed = Some(*delta);
    });

    if observed != Some(first) {
        return false;
    }

    if head() != before_head || count() != before_count {
        return false;
    }

    // ---------------------------------------------------------------------
    // STAGE 04: observer blocks mutation
    // ---------------------------------------------------------------------

    let mut push_blocked = false;
    let mut pop_blocked = false;
    let mut clear_blocked = false;
    let mut reserve_blocked = false;

    for_each(|_| {
        push_blocked = matches!(push(first), Err(DeltaError::Busy));
        pop_blocked = pop().is_none();

        let before = count();
        clear_blocked = count() == before;

        reserve_blocked = matches!(reserve(), Err(DeltaError::Busy));
    });

    if !push_blocked || !pop_blocked || !clear_blocked || !reserve_blocked {
        return false;
    }

    if count() != 1 || head() != 0 {
        return false;
    }

    // ---------------------------------------------------------------------
    // STAGE 05: FIFO pop
    // ---------------------------------------------------------------------

    match pop() {
        Some(value) if equivalent(&value, &first) => {}
        _ => return false,
    }

    if count() != 0 || head() != 1 {
        return false;
    }

    // ---------------------------------------------------------------------
    // STAGE 06: invalid input rejection
    // ---------------------------------------------------------------------

    let invalid_object = sample_delta(
        0,
        super::semantic::ACTIVE,
        super::semantic::READY,
        super::semantic::ACTIVE,
        1,
        2,
    );

    if invalid_object.is_valid() || push(invalid_object).is_ok() {
        return false;
    }

    let invalid_semantic =
        sample_delta(11, 0, super::semantic::READY, super::semantic::ACTIVE, 1, 3);

    if invalid_semantic.is_valid() || push(invalid_semantic).is_ok() {
        return false;
    }

    let invalid_previous = sample_delta(
        12,
        super::semantic::ACTIVE,
        0,
        super::semantic::ACTIVE,
        1,
        4,
    );

    if invalid_previous.is_valid() || push(invalid_previous).is_ok() {
        return false;
    }

    let invalid_new = sample_delta(13, super::semantic::ACTIVE, super::semantic::READY, 0, 1, 5);

    if invalid_new.is_valid() || push(invalid_new).is_ok() {
        return false;
    }

    let same_state = sample_delta(
        14,
        super::semantic::ACTIVE,
        super::semantic::ACTIVE,
        super::semantic::ACTIVE,
        1,
        6,
    );

    if same_state.is_valid() || push(same_state).is_ok() {
        return false;
    }

    // ---------------------------------------------------------------------
    // STAGE 07: FIFO ordering
    // ---------------------------------------------------------------------

    clear();

    let d1 = sample_delta(
        20,
        super::semantic::CONNECTED_TO,
        super::semantic::READY,
        super::semantic::ACTIVE,
        1,
        10,
    );

    let d2 = sample_delta(
        21,
        super::semantic::CONNECTED_TO,
        super::semantic::ACTIVE,
        super::semantic::PENDING,
        1,
        11,
    );

    let d3 = sample_delta(
        22,
        super::semantic::CAUSES,
        super::semantic::PENDING,
        super::semantic::COMPLETE,
        2,
        12,
    );

    if push(d1).is_err() || push(d2).is_err() || push(d3).is_err() {
        return false;
    }

    let mut ordered = [None; MAX_DELTAS];

    if collect_fifo(&mut ordered) != 3 {
        return false;
    }

    if ordered[0] != Some(d1) || ordered[1] != Some(d2) || ordered[2] != Some(d3) {
        return false;
    }

    if count() != 3 {
        return false;
    }

    // ---------------------------------------------------------------------
    // STAGE 08: physical FIFO wraparound
    // ---------------------------------------------------------------------

    clear();

    let mut expected = [None; MAX_DELTAS];
    let mut inserted = [None; MAX_DELTAS];

    let mut index = 0usize;

    while index < MAX_DELTAS {
        let delta = sample_delta(
            100 + index as u64,
            super::semantic::ACTIVE,
            super::semantic::READY,
            super::semantic::ACTIVE,
            (index % 4) as ContextId,
            100 + index as u64,
        );

        if push(delta).is_err() {
            return false;
        }

        inserted[index] = Some(delta);
        index += 1;
    }

    if count() != MAX_DELTAS {
        return false;
    }

    let mut pop_index = 0usize;

    while pop_index < 8 {
        if pop().is_none() {
            return false;
        }

        pop_index += 1;
    }

    let mut expected_index = 0usize;
    let mut source_index = 8usize;

    while source_index < MAX_DELTAS {
        expected[expected_index] = inserted[source_index];
        expected_index += 1;
        source_index += 1;
    }

    let mut wrap_index = 0usize;

    while wrap_index < 8 {
        let delta = sample_delta(
            1_000 + wrap_index as u64,
            super::semantic::ACTIVE,
            super::semantic::READY,
            super::semantic::ACTIVE,
            (wrap_index % 4) as ContextId,
            500 + wrap_index as u64,
        );

        if push(delta).is_err() {
            return false;
        }

        expected[MAX_DELTAS - 8 + wrap_index] = Some(delta);
        wrap_index += 1;
    }

    if count() != MAX_DELTAS {
        return false;
    }

    let mut actual = [None; MAX_DELTAS];

    if collect_fifo(&mut actual) != MAX_DELTAS {
        return false;
    }

    let mut compare_index = 0usize;

    while compare_index < MAX_DELTAS {
        if actual[compare_index] != expected[compare_index] {
            return false;
        }

        compare_index += 1;
    }

    // ---------------------------------------------------------------------
    // STAGE 09: full queue rejects newest
    // ---------------------------------------------------------------------

    let overflow = sample_delta(
        500,
        super::semantic::ACTIVE,
        super::semantic::READY,
        super::semantic::ACTIVE,
        99,
        999,
    );

    if push(overflow) != Err(DeltaError::Full) {
        return false;
    }

    if count() != MAX_DELTAS {
        return false;
    }

    // ---------------------------------------------------------------------
    // STAGE 10: FIFO snapshot coherence
    // ---------------------------------------------------------------------

    let snapshot_head = head();
    let snapshot_count = count();

    let mut fifo_snapshot = [None; MAX_DELTAS];

    if snapshot_fifo_into(&mut fifo_snapshot) != snapshot_count {
        return false;
    }

    if head() != snapshot_head || count() != snapshot_count {
        return false;
    }

    let mut snapshot_index = 0usize;

    while snapshot_index < snapshot_count {
        if fifo_snapshot[snapshot_index] != actual[snapshot_index] {
            return false;
        }

        snapshot_index += 1;
    }

    // ---------------------------------------------------------------------
    // STAGE 11: reservation lifecycle
    // ---------------------------------------------------------------------

    clear();

    let reservation = match reserve() {
        Ok(value) => value,
        Err(_) => return false,
    };

    if count() != 0 {
        return false;
    }

    if push(first) != Err(DeltaError::Busy) {
        return false;
    }

    if pop().is_some() {
        return false;
    }

    if reserve().is_ok() {
        return false;
    }

    if commit_reserved(reservation, first).is_err() {
        return false;
    }

    if count() != 1 {
        return false;
    }

    if cancel_reservation(reservation) {
        return false;
    }

    // ---------------------------------------------------------------------
    // STAGE 12: reservation cancellation
    // ---------------------------------------------------------------------

    clear();

    let cancellation = match reserve() {
        Ok(value) => value,
        Err(_) => return false,
    };

    if !cancel_reservation(cancellation) {
        return false;
    }

    if count() != 0 || head() != 0 {
        return false;
    }

    if push(first).is_err() {
        return false;
    }

    if count() != 1 {
        return false;
    }

    // ---------------------------------------------------------------------
    // STAGE 13: final clean reset
    // ---------------------------------------------------------------------

    clear();

    if count() != 0 || head() != 0 {
        return false;
    }

    true
}
