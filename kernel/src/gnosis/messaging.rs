//! GNOSIS bounded kernel messaging registry.
//!
//! Togara Mesh v0.1 canonical message layer.
//!
//! - no_std compatible
//! - heap-free
//! - fixed-size, deterministic
//! - transport neutral
//!
//! This module intentionally mirrors the style of `knowledge.rs` and
//! remains synchronous. Concurrency and persistence will be handled by the
//! GNOSIS execution model in the future.

// Suggested limits for v0.1
const MAX_MESSAGES: usize = 32;
const NODE_ID_SIZE: usize = 32;
const MESSAGE_TYPE_SIZE: usize = 24;
const PAYLOAD_SIZE: usize = 192;

#[derive(Clone, Copy)]
struct MessageEntry {
    id: u64,
    source: [u8; NODE_ID_SIZE],
    source_len: usize,
    destination: [u8; NODE_ID_SIZE],
    destination_len: usize,
    msg_type: [u8; MESSAGE_TYPE_SIZE],
    msg_type_len: usize,
    sequence: u64,
    ttl: u8,
    payload: [u8; PAYLOAD_SIZE],
    payload_len: usize,
    occupied: bool,
}

impl MessageEntry {
    const fn empty() -> Self {
        Self {
            id: 0,
            source: [0; NODE_ID_SIZE],
            source_len: 0,
            destination: [0; NODE_ID_SIZE],
            destination_len: 0,
            msg_type: [0; MESSAGE_TYPE_SIZE],
            msg_type_len: 0,
            sequence: 0,
            ttl: 0,
            payload: [0; PAYLOAD_SIZE],
            payload_len: 0,
            occupied: false,
        }
    }
}

static mut MESSAGES: [MessageEntry; MAX_MESSAGES] = [MessageEntry::empty(); MAX_MESSAGES];
static mut NEXT_ID: u64 = 1;

/// Public constants describing storage limits.
pub const CAPACITY: usize = MAX_MESSAGES;
pub const NODE_ID_MAX: usize = NODE_ID_SIZE;
pub const MESSAGE_TYPE_MAX: usize = MESSAGE_TYPE_SIZE;
pub const MAX_PAYLOAD_SIZE: usize = PAYLOAD_SIZE;

/// Current number of occupied message slots.
pub fn count() -> usize {
    let mut c = 0;

    unsafe {
        while c < MAX_MESSAGES {
            if MESSAGES[c].occupied {
                c += 1;
            } else {
                break;
            }
        }
    }

    c
}

/// Determine whether a message exists by stable ID.
pub fn contains(id: u64) -> bool {
    unsafe {
        let mut i = 0;
        while i < MAX_MESSAGES {
            if MESSAGES[i].occupied && MESSAGES[i].id == id {
                return true;
            }
            i += 1;
        }
    }

    false
}

/// Insert (send) a new message.
///
/// Parameters are provided as byte slices for node IDs and message type
/// to remain transport- and encoding-neutral.
pub fn send(
    source: &[u8],
    destination: &[u8],
    msg_type: &[u8],
    sequence: u64,
    ttl: u8,
    payload: &[u8],
) -> Option<u64> {
    if source.len() == 0
        || source.len() > NODE_ID_SIZE
        || destination.len() == 0
        || destination.len() > NODE_ID_SIZE
        || msg_type.len() == 0
        || msg_type.len() > MESSAGE_TYPE_SIZE
        || payload.len() > PAYLOAD_SIZE
        || ttl == 0
    {
        return None;
    }

    unsafe {
        let mut index = 0;
        while index < MAX_MESSAGES {
            if !MESSAGES[index].occupied {
                let id = NEXT_ID;
                NEXT_ID = NEXT_ID.wrapping_add(1);

                // zero out fixed arrays before copy
                MESSAGES[index].source = [0; NODE_ID_SIZE];
                MESSAGES[index].destination = [0; NODE_ID_SIZE];
                MESSAGES[index].msg_type = [0; MESSAGE_TYPE_SIZE];
                MESSAGES[index].payload = [0; PAYLOAD_SIZE];

                let mut i = 0;
                while i < source.len() {
                    MESSAGES[index].source[i] = source[i];
                    i += 1;
                }

                let mut j = 0;
                while j < destination.len() {
                    MESSAGES[index].destination[j] = destination[j];
                    j += 1;
                }

                let mut k = 0;
                while k < msg_type.len() {
                    MESSAGES[index].msg_type[k] = msg_type[k];
                    k += 1;
                }

                let mut p = 0;
                while p < payload.len() {
                    MESSAGES[index].payload[p] = payload[p];
                    p += 1;
                }

                MESSAGES[index].id = id;
                MESSAGES[index].source_len = source.len();
                MESSAGES[index].destination_len = destination.len();
                MESSAGES[index].msg_type_len = msg_type.len();
                MESSAGES[index].sequence = sequence;
                MESSAGES[index].ttl = ttl;
                MESSAGES[index].payload_len = payload.len();
                MESSAGES[index].occupied = true;

                return Some(id);
            }

            index += 1;
        }
    }

    None
}

/// Import an externally-sourced message with a supplied stable `id`.
///
/// This accepts externally created message IDs (for mesh relays). It
/// rejects `id == 0`, duplicate IDs, oversized fields, and zero `ttl`.
/// On success stores the exact supplied `id` and returns `Some(id)`.
pub fn import_with_id(
    id: u64,
    source: &[u8],
    destination: &[u8],
    msg_type: &[u8],
    sequence: u64,
    ttl: u8,
    payload: &[u8],
) -> Option<u64> {
    if id == 0
        || source.len() == 0
        || source.len() > NODE_ID_SIZE
        || destination.len() == 0
        || destination.len() > NODE_ID_SIZE
        || msg_type.len() == 0
        || msg_type.len() > MESSAGE_TYPE_SIZE
        || payload.len() > PAYLOAD_SIZE
        || ttl == 0
    {
        return None;
    }

    // Reject duplicates
    if contains(id) {
        return None;
    }

    unsafe {
        let mut index = 0;
        while index < MAX_MESSAGES {
            if !MESSAGES[index].occupied {
                // zero out fixed arrays before copy
                MESSAGES[index].source = [0; NODE_ID_SIZE];
                MESSAGES[index].destination = [0; NODE_ID_SIZE];
                MESSAGES[index].msg_type = [0; MESSAGE_TYPE_SIZE];
                MESSAGES[index].payload = [0; PAYLOAD_SIZE];

                let mut i = 0;
                while i < source.len() {
                    MESSAGES[index].source[i] = source[i];
                    i += 1;
                }

                let mut j = 0;
                while j < destination.len() {
                    MESSAGES[index].destination[j] = destination[j];
                    j += 1;
                }

                let mut k = 0;
                while k < msg_type.len() {
                    MESSAGES[index].msg_type[k] = msg_type[k];
                    k += 1;
                }

                let mut p = 0;
                while p < payload.len() {
                    MESSAGES[index].payload[p] = payload[p];
                    p += 1;
                }

                MESSAGES[index].id = id;
                MESSAGES[index].source_len = source.len();
                MESSAGES[index].destination_len = destination.len();
                MESSAGES[index].msg_type_len = msg_type.len();
                MESSAGES[index].sequence = sequence;
                MESSAGES[index].ttl = ttl;
                MESSAGES[index].payload_len = payload.len();
                MESSAGES[index].occupied = true;

                // Do not update NEXT_ID; imported IDs are external.
                return Some(id);
            }

            index += 1;
        }
    }

    None
}

/// Read a message by stable ID with a callback while the registry remains
/// borrowed. Callback receives (id, source, destination, msg_type, sequence, ttl, payload).
pub fn with_message<R>(
    id: u64,
    callback: impl FnOnce(&[u8], &[u8], &[u8], u64, u8, &[u8]) -> R,
) -> Option<R> {
    unsafe {
        let mut i = 0;
        while i < MAX_MESSAGES {
            if MESSAGES[i].occupied && MESSAGES[i].id == id {
                let s_len = MESSAGES[i].source_len;
                let d_len = MESSAGES[i].destination_len;
                let t_len = MESSAGES[i].msg_type_len;
                let p_len = MESSAGES[i].payload_len;

                return Some(callback(
                    &MESSAGES[i].source[..s_len],
                    &MESSAGES[i].destination[..d_len],
                    &MESSAGES[i].msg_type[..t_len],
                    MESSAGES[i].sequence,
                    MESSAGES[i].ttl,
                    &MESSAGES[i].payload[..p_len],
                ));
            }
            i += 1;
        }
    }

    None
}

/// Relay (forward) a message by decreasing its TTL by one.
/// Returns true on success. Does not create new messages or change the ID.
pub fn relay(id: u64) -> bool {
    unsafe {
        let mut i = 0;
        while i < MAX_MESSAGES {
            if MESSAGES[i].occupied && MESSAGES[i].id == id {
                if MESSAGES[i].ttl == 0 {
                    return false;
                }

                MESSAGES[i].ttl = MESSAGES[i].ttl.wrapping_sub(1);
                return true;
            }
            i += 1;
        }
    }

    false
}

/// Remove a message by ID. Returns true when an occupied slot was cleared.
pub fn remove(id: u64) -> bool {
    unsafe {
        let mut i = 0;
        while i < MAX_MESSAGES {
            if MESSAGES[i].occupied && MESSAGES[i].id == id {
                MESSAGES[i] = MessageEntry::empty();
                return true;
            }
            i += 1;
        }
    }

    false
}

/// Clear the entire registry (internal use for self-test).
fn clear() {
    unsafe {
        let mut i = 0;
        while i < MAX_MESSAGES {
            MESSAGES[i] = MessageEntry::empty();
            i += 1;
        }
        NEXT_ID = 1;
    }
}

/// Deterministic self-test covering insertion, read, TTL, relay, removal, and limits.
pub fn self_test() -> bool {
    clear();

    // Basic message parts
    let src = b"SOURCE_NODE_1234567890";
    let dst = b"DEST_NODE_ABCDEFGHIJKL";
    let mtype = b"TEXT";
    let payload = b"HELLO TOGARA MESH";

    // Oversized inputs should be rejected
    let oversized_payload = [0u8; PAYLOAD_SIZE + 1];
    if send(src, dst, mtype, 1, 4, &oversized_payload).is_some() {
        clear();
        return false;
    }

    // Insert a message
    let id = match send(src, dst, mtype, 42, 4, payload) {
        Some(id) => id,
        None => {
            clear();
            return false;
        }
    };

    // Count and contains
    if count() != 1 || !contains(id) {
        clear();
        return false;
    }

    // Read and validate contents
    let mut ok = false;
    let _ = with_message(id, |s, d, t, seq, ttl, p| {
        ok = bytes_equal(s, src)
            && bytes_equal(d, dst)
            && bytes_equal(t, mtype)
            && seq == 42
            && ttl == 4
            && bytes_equal(p, payload);
    });

    if !ok {
        clear();
        return false;
    }

    // Relay decrements TTL but preserves ID and contents
    let pre_id = id;
    if !relay(id) {
        clear();
        return false;
    }

    // After relay TTL should be 3
    let mut observed_ttl = 0;
    let _ = with_message(id, |_s, _d, _t, _seq, ttl, _p| {
        observed_ttl = ttl as u8;
    });

    if observed_ttl != 3 {
        clear();
        return false;
    }

    if !contains(pre_id) {
        clear();
        return false;
    }

    // Relaying until TTL reaches zero then ensuring further relay is rejected
    if !relay(id) || !relay(id) {
        clear();
        return false;
    }

    // TTL should now be 1 -> after one more relay becomes 0
    if !relay(id) {
        clear();
        return false;
    }

    // Now TTL is zero; further relay should fail
    if relay(id) {
        clear();
        return false;
    }

    // Removal
    if !remove(id) {
        clear();
        return false;
    }

    if contains(id) || count() != 0 {
        clear();
        return false;
    }

    // Fill capacity to ensure boundary behavior
    let mut ids = [0u64; MAX_MESSAGES];
    let mut created = 0usize;
    let mut idx = 0usize;
    while idx < MAX_MESSAGES {
        let temp_id = match send(src, dst, mtype, idx as u64, 1, payload) {
            Some(i) => i,
            None => break,
        };
        ids[idx] = temp_id;
        created += 1;
        idx += 1;
    }

    if created != MAX_MESSAGES {
        clear();
        return false;
    }

    // Now the registry is full; additional insert should fail
    if send(src, dst, mtype, 999, 1, payload).is_some() {
        clear();
        return false;
    }

    // Free exactly one occupied slot so import_with_id can succeed.
    // Remove the first created id to free a slot.
    if !remove(ids[0]) {
        clear();
        return false;
    }

    // Test import_with_id: import an externally supplied stable ID
    let import_id: u64 = 9999;
    if import_with_id(import_id, src, dst, mtype, 7, 2, payload) != Some(import_id) {
        clear();
        return false;
    }

    // contains works
    if !contains(import_id) {
        clear();
        return false;
    }

    // duplicate import is rejected and does not increase count
    let before = count();
    if import_with_id(import_id, src, dst, mtype, 7, 2, payload).is_some() {
        clear();
        return false;
    }

    if count() != before {
        clear();
        return false;
    }

    // imported message can be relayed
    if !relay(import_id) {
        clear();
        return false;
    }

    // relay preserves ID and decrements TTL
    let mut observed_ttl2 = 0;
    let _ = with_message(import_id, |_s, _d, _t, _seq, ttl, _p| {
        observed_ttl2 = ttl as u8;
    });

    if observed_ttl2 != 1 {
        clear();
        return false;
    }

    // Clear and verify
    clear();
    count() == 0 && !contains(1)
}

#[inline]
fn bytes_equal(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }

    true
}
