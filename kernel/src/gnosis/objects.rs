//! GNOSIS Knowledge Object (KOBJ) v1 — canonical representation, registry,
//! validation, and bounded canonical state transitions.
//!
//! KOBJ is canonical state.
//! Derived GNOSIS layers must not mutate this registry.
//!
//! Canonical transitions are prepared and committed through this module.
//! Publication of the resulting SemanticDelta is owned by the transition
//! boundary so canonical mutation cannot be treated as complete without
//! publication.

use spin::Mutex;

use super::semantic::{ContextId, ObjectId, SemanticId};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KnowledgeObject {
    pub id: ObjectId,
    pub semantic_type: SemanticId,
    pub state: SemanticId,
    pub context: ContextId,
    pub flags: u16,
    pub version: u16,
}

pub const FLAG_ACTIVE: u16 = 1 << 0;
pub const FLAG_IMMUTABLE: u16 = 1 << 1;
pub const FLAG_DERIVED: u16 = 1 << 2;
pub const FLAG_VERIFIED: u16 = 1 << 3;
pub const FLAG_PRIVATE: u16 = 1 << 4;
pub const FLAG_REMOTE: u16 = 1 << 5;

pub const CURRENT_VERSION: u16 = 1;
pub const MAX_OBJECTS: usize = 64;
pub const CAPACITY: usize = MAX_OBJECTS;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateTransition {
    pub object_id: ObjectId,
    pub semantic_id: SemanticId,
    pub previous_state: SemanticId,
    pub new_state: SemanticId,
    pub context: ContextId,
    pub timestamp: u64,
    pub version: u16,
}

#[derive(Clone, Copy)]
struct ObjectEntry {
    object: KnowledgeObject,
    occupied: bool,
}

impl ObjectEntry {
    const fn empty() -> Self {
        Self {
            object: KnowledgeObject {
                id: 0,
                semantic_type: 0,
                state: 0,
                context: 0,
                flags: 0,
                version: 0,
            },
            occupied: false,
        }
    }
}

static OBJECTS: Mutex<[ObjectEntry; MAX_OBJECTS]> = Mutex::new([ObjectEntry::empty(); MAX_OBJECTS]);

#[inline]
pub const fn is_valid(object: &KnowledgeObject) -> bool {
    object.id != 0
        && super::semantic::is_valid(object.semantic_type)
        && super::semantic::is_valid(object.state)
        && object.version != 0
}

#[inline]
pub fn count() -> usize {
    let objects = OBJECTS.lock();

    let mut count = 0usize;
    let mut index = 0usize;

    while index < MAX_OBJECTS {
        if objects[index].occupied {
            count += 1;
        }
        index += 1;
    }

    count
}

#[inline]
pub fn contains(id: ObjectId) -> bool {
    if id == 0 {
        return false;
    }

    let objects = OBJECTS.lock();

    let mut index = 0usize;
    while index < MAX_OBJECTS {
        if objects[index].occupied && objects[index].object.id == id {
            return true;
        }
        index += 1;
    }

    false
}

#[inline]
pub fn insert(object: KnowledgeObject) -> bool {
    if !is_valid(&object) {
        return false;
    }

    let mut objects = OBJECTS.lock();

    let mut index = 0usize;
    while index < MAX_OBJECTS {
        if objects[index].occupied && objects[index].object.id == object.id {
            return false;
        }
        index += 1;
    }

    index = 0;
    while index < MAX_OBJECTS {
        if !objects[index].occupied {
            objects[index] = ObjectEntry {
                object,
                occupied: true,
            };
            return true;
        }
        index += 1;
    }

    false
}

#[inline]
pub fn with_object<R, F: FnOnce(&KnowledgeObject) -> R>(id: ObjectId, callback: F) -> Option<R> {
    if id == 0 {
        return None;
    }

    let object = {
        let objects = OBJECTS.lock();

        let mut index = 0usize;
        let mut found = None;

        while index < MAX_OBJECTS {
            if objects[index].occupied && objects[index].object.id == id {
                found = Some(objects[index].object);
                break;
            }
            index += 1;
        }

        found
    };

    object.as_ref().map(callback)
}

#[inline]
pub fn for_each<F: FnMut(&KnowledgeObject)>(mut callback: F) {
    let snapshot = {
        let objects = OBJECTS.lock();

        let mut values = [KnowledgeObject {
            id: 0,
            semantic_type: 0,
            state: 0,
            context: 0,
            flags: 0,
            version: 0,
        }; MAX_OBJECTS];

        let mut count = 0usize;
        let mut index = 0usize;

        while index < MAX_OBJECTS {
            if objects[index].occupied {
                values[count] = objects[index].object;
                count += 1;
            }
            index += 1;
        }

        (values, count)
    };

    let (values, count) = snapshot;

    let mut index = 0usize;
    while index < count {
        callback(&values[index]);
        index += 1;
    }
}

/// Prepare a canonical transition without mutating the object registry.
///
/// This is intentionally crate-visible. Callers must commit the returned
/// transition through the transition boundary so the canonical mutation and
/// SemanticDelta publication remain one lifecycle.
#[inline]
pub(crate) fn prepare_transition(
    id: ObjectId,
    new_state: SemanticId,
    timestamp: u64,
) -> Option<StateTransition> {
    if id == 0 || !super::semantic::is_valid(new_state) {
        return None;
    }

    let objects = OBJECTS.lock();

    let mut index = 0usize;
    while index < MAX_OBJECTS {
        if objects[index].occupied && objects[index].object.id == id {
            let object = objects[index].object;

            if (object.flags & FLAG_IMMUTABLE) != 0 || object.state == new_state {
                return None;
            }

            if object.version == u16::MAX {
                return None;
            }

            let next_version = object.version + 1;

            return Some(StateTransition {
                object_id: object.id,
                semantic_id: object.semantic_type,
                previous_state: object.state,
                new_state,
                context: object.context,
                timestamp,
                version: next_version,
            });
        }

        index += 1;
    }

    None
}

/// Commit a previously prepared canonical transition.
///
/// The object must still exactly match the prepared previous state/version.
/// This prevents a stale transition record from mutating canonical state.
#[inline]
pub(crate) fn commit_transition(transition: StateTransition) -> bool {
    if transition.object_id == 0
        || !super::semantic::is_valid(transition.semantic_id)
        || !super::semantic::is_valid(transition.previous_state)
        || !super::semantic::is_valid(transition.new_state)
        || transition.previous_state == transition.new_state
        || transition.version == 0
    {
        return false;
    }

    let mut objects = OBJECTS.lock();

    let mut index = 0usize;
    while index < MAX_OBJECTS {
        if objects[index].occupied && objects[index].object.id == transition.object_id {
            let object = objects[index].object;

            if (object.flags & FLAG_IMMUTABLE) != 0 {
                return false;
            }

            if object.semantic_type != transition.semantic_id
                || object.state != transition.previous_state
                || object.context != transition.context
                || object.version == u16::MAX
                || object.version + 1 != transition.version
            {
                return false;
            }

            objects[index].object.state = transition.new_state;
            objects[index].object.version = transition.version;

            return true;
        }

        index += 1;
    }

    false
}

#[inline]
pub fn remove(id: ObjectId) -> bool {
    if id == 0 {
        return false;
    }

    let mut objects = OBJECTS.lock();

    let mut index = 0usize;
    while index < MAX_OBJECTS {
        if objects[index].occupied && objects[index].object.id == id {
            objects[index] = ObjectEntry::empty();
            return true;
        }
        index += 1;
    }

    false
}

#[inline]
pub fn clear() {
    let mut objects = OBJECTS.lock();
    *objects = [ObjectEntry::empty(); MAX_OBJECTS];
}

#[inline]
pub fn self_test() -> bool {
    clear();

    let object = KnowledgeObject {
        id: 1,
        semantic_type: super::semantic::STATE_OBJECT,
        state: super::semantic::READY,
        context: 7,
        flags: FLAG_ACTIVE,
        version: CURRENT_VERSION,
    };

    if !is_valid(&object) || !insert(object) || count() != 1 || !contains(object.id) {
        clear();
        return false;
    }

    if insert(object) {
        clear();
        return false;
    }

    let mut observed = None;
    with_object(object.id, |value| observed = Some(*value));

    if observed != Some(object) {
        clear();
        return false;
    }

    if prepare_transition(object.id, object.state, 10).is_some() {
        clear();
        return false;
    }

    let prepared = match prepare_transition(object.id, super::semantic::ACTIVE, 11) {
        Some(value) => value,
        None => {
            clear();
            return false;
        }
    };

    if prepared.object_id != object.id
        || prepared.semantic_id != object.semantic_type
        || prepared.previous_state != object.state
        || prepared.new_state != super::semantic::ACTIVE
        || prepared.context != object.context
        || prepared.timestamp != 11
        || prepared.version != object.version + 1
    {
        clear();
        return false;
    }

    if !commit_transition(prepared) {
        clear();
        return false;
    }

    let mut updated = None;
    with_object(object.id, |value| updated = Some(*value));

    if updated.map(|value| value.state) != Some(super::semantic::ACTIVE)
        || updated.map(|value| value.version) != Some(prepared.version)
    {
        clear();
        return false;
    }

    if commit_transition(prepared) {
        clear();
        return false;
    }

    let immutable = KnowledgeObject {
        id: 2,
        semantic_type: super::semantic::STATE_OBJECT,
        state: super::semantic::READY,
        context: 8,
        flags: FLAG_IMMUTABLE,
        version: CURRENT_VERSION,
    };

    if !insert(immutable) || prepare_transition(immutable.id, super::semantic::ACTIVE, 12).is_some()
    {
        clear();
        return false;
    }

    let stale = StateTransition {
        object_id: object.id,
        semantic_id: object.semantic_type,
        previous_state: super::semantic::READY,
        new_state: super::semantic::ACTIVE,
        context: object.context,
        timestamp: 99,
        version: prepared.version + 1,
    };

    if commit_transition(stale) {
        clear();
        return false;
    }

    if !remove(object.id) || contains(object.id) || count() != 1 {
        clear();
        return false;
    }

    clear();
    count() == 0 && !contains(1)
}
