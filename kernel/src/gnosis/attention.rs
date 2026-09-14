use super::delta::SemanticDelta;
use super::relation::{ProvenanceSource, RelationProvenance};
use super::semantic::{ContextId, ObjectId, SemanticId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttentionError {
    Invalid,
    Full,
}

#[derive(Clone, Copy)]
pub struct AttentionRecord {
    pub object_id: ObjectId,
    pub semantic_id: SemanticId,
    pub previous_state: SemanticId,
    pub new_state: SemanticId,
    pub context: ContextId,
    pub timestamp: u64,
    pub provenance: RelationProvenance,
    pub score: u16,
}

pub const MAX_ATTENTION: usize = 64;

#[inline]
pub const fn capacity() -> usize {
    MAX_ATTENTION
}

static mut ATTENTION_QUEUE: [Option<AttentionRecord>; MAX_ATTENTION] = [None; MAX_ATTENTION];
static mut ATTENTION_HEAD: usize = 0;
static mut ATTENTION_COUNT: usize = 0;

#[inline]
fn current_ticks() -> u64 {
    crate::system::state::uptime_ticks()
}

#[inline]
fn transition_weight(delta: &SemanticDelta) -> u16 {
    if delta.new_state == super::semantic::FAILED
        || delta.new_state == super::semantic::INVALID
        || delta.new_state == super::semantic::UNAUTHORIZED
    {
        return 400;
    }

    if delta.new_state == super::semantic::EXPIRED
        || delta.new_state == super::semantic::UNAVAILABLE
        || delta.new_state == super::semantic::OFFLINE
        || delta.new_state == super::semantic::STOPPED
    {
        return 250;
    }

    100
}

#[inline]
fn semantic_weight(delta: &SemanticDelta) -> u16 {
    match delta.semantic_id {
        super::semantic::AUTHORIZATION
        | super::semantic::AUTHENTICATION
        | super::semantic::PERMISSION
        | super::semantic::CAPABILITY
        | super::semantic::TRUST
        | super::semantic::INTEGRITY
        | super::semantic::CONFIDENTIALITY
        | super::semantic::PRIVACY
        | super::semantic::REVOKE
        | super::semantic::EXPIRE => 250,
        super::semantic::CONNECT
        | super::semantic::DISCONNECT
        | super::semantic::SEND
        | super::semantic::RECEIVE
        | super::semantic::RELAY
        | super::semantic::FORWARD
        | super::semantic::ROUTES_TO
        | super::semantic::RECEIVES_FROM
        | super::semantic::SENDS_TO
        | super::semantic::CONNECTED_TO
        | super::semantic::DISCONNECTED_FROM
        | super::semantic::PACKET
        | super::semantic::FRAME
        | super::semantic::ROUTE
        | super::semantic::CHANNEL
        | super::semantic::QUEUE
        | super::semantic::TTL
        | super::semantic::SEQUENCE => 120,
        _ => 0,
    }
}

#[inline]
fn freshness_weight(delta: &SemanticDelta) -> u16 {
    let age = current_ticks().saturating_sub(delta.timestamp);
    let recent = crate::interrupts::timer::TIMER_FREQUENCY as u64 / 2;
    let moderate = crate::interrupts::timer::TIMER_FREQUENCY as u64 * 5;

    if age == 0 {
        250
    } else if age <= recent {
        150
    } else if age <= moderate {
        75
    } else {
        0
    }
}

#[inline]
fn provenance_weight(delta: &SemanticDelta) -> u16 {
    match delta.provenance.source {
        ProvenanceSource::Local => 100,
        ProvenanceSource::Remote => 50,
        ProvenanceSource::Recovered => 25,
    }
}

#[inline]
fn total_score(delta: &SemanticDelta) -> u16 {
    let score = u32::from(transition_weight(delta))
        + u32::from(semantic_weight(delta))
        + u32::from(freshness_weight(delta))
        + u32::from(provenance_weight(delta));

    if score > 1000 { 1000 } else { score as u16 }
}

#[inline]
pub fn count() -> usize {
    unsafe { ATTENTION_COUNT }
}

/// Visits every occupied attention record in deterministic queue order without
/// consuming or mutating the registry.
pub fn for_each<F: FnMut(&AttentionRecord)>(mut callback: F) {
    unsafe {
        let mut index = 0usize;
        while index < ATTENTION_COUNT {
            let slot = (ATTENTION_HEAD + index) % MAX_ATTENTION;
            if let Some(record) = ATTENTION_QUEUE[slot] {
                callback(&record);
            }
            index += 1;
        }
    }
}

#[inline]
pub fn clear() {
    unsafe {
        ATTENTION_QUEUE = [None; MAX_ATTENTION];
        ATTENTION_HEAD = 0;
        ATTENTION_COUNT = 0;
    }
}

#[inline]
pub fn evaluate(delta: &SemanticDelta) -> Option<AttentionRecord> {
    if !delta.is_valid() {
        return None;
    }

    let score = total_score(delta);

    Some(AttentionRecord {
        object_id: delta.object_id,
        semantic_id: delta.semantic_id,
        previous_state: delta.previous_state,
        new_state: delta.new_state,
        context: delta.context,
        timestamp: delta.timestamp,
        provenance: delta.provenance,
        score,
    })
}

#[inline]
pub fn push(delta: SemanticDelta) -> Result<usize, AttentionError> {
    if !delta.is_valid() {
        return Err(AttentionError::Invalid);
    }

    let record = match evaluate(&delta) {
        Some(value) => value,
        None => return Err(AttentionError::Invalid),
    };

    unsafe {
        if ATTENTION_COUNT >= MAX_ATTENTION {
            return Err(AttentionError::Full);
        }

        let index = (ATTENTION_HEAD + ATTENTION_COUNT) % MAX_ATTENTION;
        ATTENTION_QUEUE[index] = Some(record);
        ATTENTION_COUNT += 1;
        Ok(index)
    }
}

#[inline]
pub fn pop() -> Option<AttentionRecord> {
    unsafe {
        if ATTENTION_COUNT == 0 {
            return None;
        }

        let value = ATTENTION_QUEUE[ATTENTION_HEAD];
        ATTENTION_QUEUE[ATTENTION_HEAD] = None;
        ATTENTION_HEAD = (ATTENTION_HEAD + 1) % MAX_ATTENTION;
        ATTENTION_COUNT -= 1;
        value
    }
}

#[inline]
fn make_delta(
    object_id: ObjectId,
    semantic_id: SemanticId,
    previous_state: SemanticId,
    new_state: SemanticId,
    context: ContextId,
    timestamp: u64,
    origin: u64,
    source: ProvenanceSource,
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
            origin,
            source,
            revision,
        },
    }
}

#[inline]
pub fn self_test() -> bool {
    clear();

    let mut ok = true;

    if count() != 0 {
        ok = false;
    }

    // Use a deterministic synthetic timestamp so the self-test cannot race
    // the BIOS timer between score calculation and evaluation.
    // `current_ticks().saturating_sub(u64::MAX)` is always zero.
    let now = u64::MAX;
    let compute_expected_score = |delta: &SemanticDelta| -> u16 {
        let transition = if delta.new_state == super::semantic::FAILED
            || delta.new_state == super::semantic::INVALID
            || delta.new_state == super::semantic::UNAUTHORIZED
        {
            400u32
        } else if delta.new_state == super::semantic::EXPIRED
            || delta.new_state == super::semantic::UNAVAILABLE
            || delta.new_state == super::semantic::OFFLINE
            || delta.new_state == super::semantic::STOPPED
        {
            250u32
        } else {
            100u32
        };

        let semantic = match delta.semantic_id {
            super::semantic::AUTHORIZATION
            | super::semantic::AUTHENTICATION
            | super::semantic::PERMISSION
            | super::semantic::CAPABILITY
            | super::semantic::TRUST
            | super::semantic::INTEGRITY
            | super::semantic::CONFIDENTIALITY
            | super::semantic::PRIVACY
            | super::semantic::REVOKE
            | super::semantic::EXPIRE => 250u32,
            super::semantic::CONNECT
            | super::semantic::DISCONNECT
            | super::semantic::SEND
            | super::semantic::RECEIVE
            | super::semantic::RELAY
            | super::semantic::FORWARD
            | super::semantic::ROUTES_TO
            | super::semantic::RECEIVES_FROM
            | super::semantic::SENDS_TO
            | super::semantic::CONNECTED_TO
            | super::semantic::DISCONNECTED_FROM
            | super::semantic::PACKET
            | super::semantic::FRAME
            | super::semantic::ROUTE
            | super::semantic::CHANNEL
            | super::semantic::QUEUE
            | super::semantic::TTL
            | super::semantic::SEQUENCE => 120u32,
            _ => 0u32,
        };

        let age = current_ticks().saturating_sub(delta.timestamp);
        let recent = crate::interrupts::timer::TIMER_FREQUENCY as u64 / 2;
        let moderate = crate::interrupts::timer::TIMER_FREQUENCY as u64 * 5;
        let freshness = if age == 0 {
            250u32
        } else if age <= recent {
            150u32
        } else if age <= moderate {
            75u32
        } else {
            0u32
        };

        let provenance = match delta.provenance.source {
            ProvenanceSource::Local => 100u32,
            ProvenanceSource::Remote => 50u32,
            ProvenanceSource::Recovered => 25u32,
        };

        let score = transition + semantic + freshness + provenance;
        if score > 1000 { 1000 } else { score as u16 }
    };

    let valid = make_delta(
        42,
        super::semantic::AUTHORIZATION,
        super::semantic::READY,
        super::semantic::FAILED,
        9,
        now,
        77,
        ProvenanceSource::Local,
        2,
    );

    if !valid.is_valid() {
        ok = false;
    }

    let valid_expected = compute_expected_score(&valid);
    match evaluate(&valid) {
        Some(record) => {
            if record.object_id != valid.object_id
                || record.semantic_id != valid.semantic_id
                || record.previous_state != valid.previous_state
                || record.new_state != valid.new_state
                || record.context != valid.context
                || record.timestamp != valid.timestamp
                || record.provenance.origin != valid.provenance.origin
                || record.provenance.source != valid.provenance.source
                || record.provenance.revision != valid.provenance.revision
                || record.score != valid_expected
            {
                ok = false;
            }
        }
        None => ok = false,
    }

    let invalid_object = make_delta(
        0,
        super::semantic::AUTHORIZATION,
        super::semantic::READY,
        super::semantic::FAILED,
        3,
        now,
        1,
        ProvenanceSource::Local,
        1,
    );
    if invalid_object.is_valid() || evaluate(&invalid_object).is_some() {
        ok = false;
    }

    let invalid_semantic = make_delta(
        11,
        0,
        super::semantic::READY,
        super::semantic::FAILED,
        3,
        now,
        2,
        ProvenanceSource::Remote,
        1,
    );
    if invalid_semantic.is_valid() || evaluate(&invalid_semantic).is_some() {
        ok = false;
    }

    let invalid_previous = make_delta(
        12,
        super::semantic::AUTHORIZATION,
        0,
        super::semantic::FAILED,
        3,
        now,
        3,
        ProvenanceSource::Recovered,
        1,
    );
    if invalid_previous.is_valid() || evaluate(&invalid_previous).is_some() {
        ok = false;
    }

    let invalid_new = make_delta(
        13,
        super::semantic::AUTHORIZATION,
        super::semantic::READY,
        0,
        3,
        now,
        4,
        ProvenanceSource::Local,
        1,
    );
    if invalid_new.is_valid() || evaluate(&invalid_new).is_some() {
        ok = false;
    }

    let same_state = make_delta(
        14,
        super::semantic::AUTHORIZATION,
        super::semantic::READY,
        super::semantic::READY,
        3,
        now,
        5,
        ProvenanceSource::Local,
        1,
    );
    if same_state.is_valid() || evaluate(&same_state).is_some() {
        ok = false;
    }

    let zero_timestamp = make_delta(
        15,
        super::semantic::AUTHORIZATION,
        super::semantic::READY,
        super::semantic::FAILED,
        0,
        0,
        6,
        ProvenanceSource::Local,
        1,
    );
    if !zero_timestamp.is_valid() || evaluate(&zero_timestamp).is_none() {
        ok = false;
    }

    let zero_context = make_delta(
        16,
        super::semantic::AUTHORIZATION,
        super::semantic::READY,
        super::semantic::EXPIRED,
        0,
        now,
        7,
        ProvenanceSource::Local,
        1,
    );
    if !zero_context.is_valid() || evaluate(&zero_context).is_none() {
        ok = false;
    }

    let local_delta = make_delta(
        17,
        super::semantic::AUTHORIZATION,
        super::semantic::READY,
        super::semantic::FAILED,
        1,
        now,
        8,
        ProvenanceSource::Local,
        1,
    );
    let remote_delta = make_delta(
        18,
        super::semantic::AUTHORIZATION,
        super::semantic::READY,
        super::semantic::FAILED,
        1,
        now,
        9,
        ProvenanceSource::Remote,
        1,
    );
    let recovered_delta = make_delta(
        19,
        super::semantic::AUTHORIZATION,
        super::semantic::READY,
        super::semantic::FAILED,
        1,
        now,
        10,
        ProvenanceSource::Recovered,
        1,
    );

    let local_expected = compute_expected_score(&local_delta);
    let remote_expected = compute_expected_score(&remote_delta);
    let recovered_expected = compute_expected_score(&recovered_delta);

    match evaluate(&local_delta) {
        Some(record) => {
            if record.score != local_expected {
                ok = false;
            }
        }
        None => ok = false,
    }
    match evaluate(&remote_delta) {
        Some(record) => {
            if record.score != remote_expected {
                ok = false;
            }
        }
        None => ok = false,
    }
    match evaluate(&recovered_delta) {
        Some(record) => {
            if record.score != recovered_expected {
                ok = false;
            }
        }
        None => ok = false,
    }

    let mut expected_object_ids = [0u64; MAX_ATTENTION];
    let mut expected_states = [0u32; MAX_ATTENTION];
    let mut expected_sources = [ProvenanceSource::Local; MAX_ATTENTION];
    let mut expected_origins = [0u64; MAX_ATTENTION];
    let mut expected_revisions = [0u64; MAX_ATTENTION];
    let mut expected_scores = [0u16; MAX_ATTENTION];

    for i in 0..MAX_ATTENTION {
        let object_id = (i + 1) as u64;
        let state = match i % 3 {
            0 => super::semantic::FAILED,
            1 => super::semantic::EXPIRED,
            _ => super::semantic::OFFLINE,
        };
        let source = if i % 2 == 0 {
            ProvenanceSource::Local
        } else {
            ProvenanceSource::Remote
        };
        let delta = make_delta(
            object_id,
            super::semantic::AUTHORIZATION,
            super::semantic::READY,
            state,
            i as ContextId,
            now,
            1000 + i as u64,
            source,
            (i + 1) as u64,
        );

        expected_object_ids[i] = object_id;
        expected_states[i] = state;
        expected_sources[i] = source;
        expected_origins[i] = 1000 + i as u64;
        expected_revisions[i] = (i + 1) as u64;
        expected_scores[i] = compute_expected_score(&delta);

        if push(delta).is_err() {
            ok = false;
            break;
        }
    }

    if count() != MAX_ATTENTION {
        ok = false;
    }

    let overflow = make_delta(
        9999,
        super::semantic::AUTHORIZATION,
        super::semantic::READY,
        super::semantic::FAILED,
        999,
        now,
        5000,
        ProvenanceSource::Local,
        999,
    );

    match push(overflow) {
        Err(AttentionError::Full) => {}
        _ => ok = false,
    }

    if count() != MAX_ATTENTION {
        ok = false;
    }

    let mut seen = 0usize;
    while let Some(record) = pop() {
        if seen >= expected_object_ids.len() {
            ok = false;
            break;
        }

        if record.object_id != expected_object_ids[seen]
            || record.semantic_id != super::semantic::AUTHORIZATION
            || record.previous_state != super::semantic::READY
            || record.new_state != expected_states[seen]
            || record.context != seen as ContextId
            || record.timestamp != now
            || record.provenance.origin != expected_origins[seen]
            || record.provenance.source != expected_sources[seen]
            || record.provenance.revision != expected_revisions[seen]
            || record.score != expected_scores[seen]
            || record.object_id == overflow.object_id
        {
            ok = false;
        }

        seen += 1;
    }

    if seen != MAX_ATTENTION {
        ok = false;
    }
    if count() != 0 {
        ok = false;
    }

    clear();
    ok
}
