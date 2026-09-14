//! GNOSIS bounded kernel knowledge registry.
//!
//! The registry deliberately uses fixed-size storage so GNOSIS can operate
//! without depending on the heap allocator. This is the first concrete
//! knowledge substrate inside the kernel.

const MAX_ENTRIES: usize = 16;
const KEY_SIZE: usize = 32;
const VALUE_SIZE: usize = 96;

#[derive(Clone, Copy)]
struct KnowledgeEntry {
    id: u64,
    key: [u8; KEY_SIZE],
    key_len: usize,
    value: [u8; VALUE_SIZE],
    value_len: usize,
    occupied: bool,
}

impl KnowledgeEntry {
    const fn empty() -> Self {
        Self {
            id: 0,
            key: [0; KEY_SIZE],
            key_len: 0,
            value: [0; VALUE_SIZE],
            value_len: 0,
            occupied: false,
        }
    }
}

static mut ENTRIES: [KnowledgeEntry; MAX_ENTRIES] = [KnowledgeEntry::empty(); MAX_ENTRIES];

static mut NEXT_ID: u64 = 1;

/// Maximum number of knowledge entries GNOSIS can currently hold.
pub const CAPACITY: usize = MAX_ENTRIES;

/// Maximum key size accepted by the registry.
pub const MAX_KEY_SIZE: usize = KEY_SIZE;

/// Maximum value size accepted by the registry.
pub const MAX_VALUE_SIZE: usize = VALUE_SIZE;

/// Number of currently occupied knowledge entries.
pub fn entry_count() -> usize {
    let mut count = 0;

    // SAFETY:
    // The kernel currently accesses the registry synchronously through the
    // shell. Concurrency protection will be introduced with the GNOSIS
    // execution model rather than duplicating synchronization primitives here.
    unsafe {
        while count < MAX_ENTRIES {
            if ENTRIES[count].occupied {
                count += 1;
            } else {
                break;
            }
        }
    }

    count
}

/// Insert a new knowledge entry.
///
/// Returns the stable entry ID on success.
///
/// Returns `None` when:
/// - the key is empty,
/// - the key is too large,
/// - the value is too large,
/// - or the registry is full.
pub fn insert(key: &[u8], value: &[u8]) -> Option<u64> {
    if key.is_empty() || key.len() > KEY_SIZE || value.len() > VALUE_SIZE {
        return None;
    }

    unsafe {
        let mut index = 0;

        while index < MAX_ENTRIES {
            if !ENTRIES[index].occupied {
                let id = NEXT_ID;
                NEXT_ID = NEXT_ID.wrapping_add(1);

                ENTRIES[index].key = [0; KEY_SIZE];
                ENTRIES[index].value = [0; VALUE_SIZE];

                let mut i = 0;
                while i < key.len() {
                    ENTRIES[index].key[i] = key[i];
                    i += 1;
                }

                i = 0;
                while i < value.len() {
                    ENTRIES[index].value[i] = value[i];
                    i += 1;
                }

                ENTRIES[index].id = id;
                ENTRIES[index].key_len = key.len();
                ENTRIES[index].value_len = value.len();
                ENTRIES[index].occupied = true;

                return Some(id);
            }

            index += 1;
        }
    }

    None
}

/// Insert a new entry or replace the value for an existing key.
///
/// Updating an existing key preserves its stable entry ID. A new key receives
/// a new ID through the normal insertion path.
pub fn upsert(key: &[u8], value: &[u8]) -> Option<u64> {
    if key.is_empty() || key.len() > KEY_SIZE || value.len() > VALUE_SIZE {
        return None;
    }

    unsafe {
        let mut index = 0;

        while index < MAX_ENTRIES {
            if ENTRIES[index].occupied
                && ENTRIES[index].key_len == key.len()
                && bytes_equal(&ENTRIES[index].key[..ENTRIES[index].key_len], key)
            {
                ENTRIES[index].value = [0; VALUE_SIZE];

                let mut i = 0;
                while i < value.len() {
                    ENTRIES[index].value[i] = value[i];
                    i += 1;
                }

                ENTRIES[index].value_len = value.len();
                return Some(ENTRIES[index].id);
            }

            index += 1;
        }
    }

    insert(key, value)
}

/// Find an entry ID by key.
pub fn find_id(key: &[u8]) -> Option<u64> {
    unsafe {
        let mut index = 0;

        while index < MAX_ENTRIES {
            if ENTRIES[index].occupied
                && ENTRIES[index].key_len == key.len()
                && bytes_equal(&ENTRIES[index].key[..ENTRIES[index].key_len], key)
            {
                return Some(ENTRIES[index].id);
            }

            index += 1;
        }
    }

    None
}

/// Read an entry by stable ID.
///
/// The callback receives the stored key and value while the registry remains
/// borrowed. This avoids returning references whose lifetime would expose
/// the mutable static directly.
pub fn with_entry<R>(id: u64, callback: impl FnOnce(&[u8], &[u8]) -> R) -> Option<R> {
    unsafe {
        let mut index = 0;

        while index < MAX_ENTRIES {
            if ENTRIES[index].occupied && ENTRIES[index].id == id {
                let key_len = ENTRIES[index].key_len;
                let value_len = ENTRIES[index].value_len;

                return Some(callback(
                    &ENTRIES[index].key[..key_len],
                    &ENTRIES[index].value[..value_len],
                ));
            }

            index += 1;
        }
    }

    None
}

/// Determine whether a knowledge entry exists.
pub fn contains(key: &[u8]) -> bool {
    find_id(key).is_some()
}

/// Clear the entire registry.
///
/// This is intentionally kept internal for now. A public reset command can
/// be added later once GNOSIS persistence semantics are defined.
fn clear() {
    unsafe {
        let mut index = 0;

        while index < MAX_ENTRIES {
            ENTRIES[index] = KnowledgeEntry::empty();
            index += 1;
        }

        NEXT_ID = 1;
    }
}

/// Verify the registry's basic insert/find/read lifecycle.
///
/// This test uses the real registry and then restores its previous empty
/// state. It is intended as a deterministic kernel self-test.
pub fn self_test() -> bool {
    clear();

    let id = match insert(b"GNOSIS", b"KNOWLEDGE SUBSTRATE") {
        Some(id) => id,
        None => return false,
    };

    if entry_count() != 1 {
        clear();
        return false;
    }

    if find_id(b"GNOSIS") != Some(id) {
        clear();
        return false;
    }

    let mut valid = false;

    let _ = with_entry(id, |key, value| {
        valid = bytes_equal(key, b"GNOSIS") && bytes_equal(value, b"KNOWLEDGE SUBSTRATE");
    });

    clear();

    valid
}

#[inline]
fn bytes_equal(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut index = 0;

    while index < a.len() {
        if a[index] != b[index] {
            return false;
        }

        index += 1;
    }

    true
}
