#![allow(clippy::missing_safety_doc)]

//! GNOSIS Relation Engine — relation registry, metadata, and indexes.
//!
//! Fixed-size, deterministic, heap-free relation storage for the GNOSIS kernel.
//! REL-01.6 extends REL-01.1/01.2/01.3 with traversal, recursive queries,
//! pathfinding, persistence, deterministic hashing, and network-ready sync frames.
//! REL-01.7 adds provenance, trust state, integrity verification, versioned updates,
//! and deterministic conflict handling.

use super::semantic::{ContextId, ObjectId, SemanticId};

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Relation {
    pub source: ObjectId,
    pub relation: SemanticId,
    pub target: ObjectId,
    pub context: ContextId,
}

pub const fn is_valid(relation: &Relation) -> bool {
    relation.source != 0 && super::semantic::is_valid(relation.relation) && relation.target != 0
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct RelationMetadata {
    pub created_at: u64,
    pub updated_at: u64,
    pub version: u16,
    pub state: SemanticId,
}

pub const CURRENT_VERSION: u16 = 1;

/// Identifies where a relation record originated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ProvenanceSource {
    Local = 1,
    Remote = 2,
    Recovered = 3,
}

/// Trust state assigned to a relation after integrity/conflict evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum RelationTrust {
    Unverified = 1,
    Verified = 2,
    Conflict = 3,
    Invalid = 4,
}

/// Fixed-size provenance attached to every relation record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct RelationProvenance {
    pub origin: u64,
    pub source: ProvenanceSource,
    pub revision: u64,
}

const fn default_provenance() -> RelationProvenance {
    RelationProvenance {
        origin: 0,
        source: ProvenanceSource::Local,
        revision: 1,
    }
}

const fn provenance_is_valid(provenance: &RelationProvenance) -> bool {
    provenance.revision != 0
        && matches!(
            provenance.source,
            ProvenanceSource::Local | ProvenanceSource::Remote | ProvenanceSource::Recovered
        )
}

const fn trust_is_valid(trust: RelationTrust) -> bool {
    matches!(
        trust,
        RelationTrust::Unverified
            | RelationTrust::Verified
            | RelationTrust::Conflict
            | RelationTrust::Invalid
    )
}

pub const MAX_RELATIONS: usize = 64;
pub const CAPACITY: usize = MAX_RELATIONS;

#[derive(Clone, Copy)]
struct RelationEntry {
    id: u64,
    relation: Relation,
    metadata: RelationMetadata,
    provenance: RelationProvenance,
    trust: RelationTrust,
    integrity: u64,
    occupied: bool,
}

impl RelationEntry {
    const fn empty() -> Self {
        Self {
            id: 0,
            relation: Relation {
                source: 0,
                relation: 0,
                target: 0,
                context: 0,
            },
            metadata: RelationMetadata {
                created_at: 0,
                updated_at: 0,
                version: 0,
                state: 0,
            },
            provenance: RelationProvenance {
                origin: 0,
                source: ProvenanceSource::Local,
                revision: 0,
            },
            trust: RelationTrust::Invalid,
            integrity: 0,
            occupied: false,
        }
    }
}

#[derive(Clone, Copy)]
struct IndexEntry {
    key: u64,
    relation_id: u64,
    occupied: bool,
}

impl IndexEntry {
    const fn empty() -> Self {
        Self {
            key: 0,
            relation_id: 0,
            occupied: false,
        }
    }
}

static mut RELATIONS: [RelationEntry; MAX_RELATIONS] = [RelationEntry::empty(); MAX_RELATIONS];

static mut SOURCE_INDEX: [IndexEntry; MAX_RELATIONS] = [IndexEntry::empty(); MAX_RELATIONS];

static mut TARGET_INDEX: [IndexEntry; MAX_RELATIONS] = [IndexEntry::empty(); MAX_RELATIONS];

static mut SEMANTIC_INDEX: [IndexEntry; MAX_RELATIONS] = [IndexEntry::empty(); MAX_RELATIONS];

static mut NEXT_ID: u64 = 1;

fn current_ticks() -> u64 {
    crate::system::state::uptime_ticks()
}

fn default_metadata() -> RelationMetadata {
    let now = current_ticks();

    RelationMetadata {
        created_at: now,
        updated_at: now,
        version: CURRENT_VERSION,
        state: super::semantic::ACTIVE,
    }
}

const fn metadata_is_valid(metadata: &RelationMetadata) -> bool {
    metadata.version != 0 && super::semantic::is_valid(metadata.state)
}

fn integrity_hash(
    id: u64,
    relation: &Relation,
    metadata: &RelationMetadata,
    provenance: &RelationProvenance,
) -> u64 {
    let mut state = 0xcbf2_9ce4_8422_2325u64;
    fn mix(state: &mut u64, value: u8) {
        *state ^= value as u64;
        *state = state.wrapping_mul(0x0000_0100_0000_01b3u64);
    }
    for byte in id.to_le_bytes() {
        mix(&mut state, byte);
    }
    for byte in relation.source.to_le_bytes() {
        mix(&mut state, byte);
    }
    for byte in relation.relation.to_le_bytes() {
        mix(&mut state, byte);
    }
    for byte in relation.target.to_le_bytes() {
        mix(&mut state, byte);
    }
    for byte in relation.context.to_le_bytes() {
        mix(&mut state, byte);
    }
    for byte in metadata.created_at.to_le_bytes() {
        mix(&mut state, byte);
    }
    for byte in metadata.updated_at.to_le_bytes() {
        mix(&mut state, byte);
    }
    for byte in metadata.version.to_le_bytes() {
        mix(&mut state, byte);
    }
    for byte in metadata.state.to_le_bytes() {
        mix(&mut state, byte);
    }
    for byte in provenance.origin.to_le_bytes() {
        mix(&mut state, byte);
    }
    mix(&mut state, provenance.source as u8);
    for byte in provenance.revision.to_le_bytes() {
        mix(&mut state, byte);
    }
    state
}

fn relation_entry_integrity_valid(entry: &RelationEntry) -> bool {
    entry.occupied
        && integrity_hash(
            entry.id,
            &entry.relation,
            &entry.metadata,
            &entry.provenance,
        ) == entry.integrity
}

unsafe fn index_insert(index: *mut IndexEntry, key: u64, relation_id: u64) -> bool {
    let mut slot = 0usize;

    while slot < MAX_RELATIONS {
        let entry = unsafe { index.add(slot) };

        if unsafe { !(*entry).occupied } {
            unsafe {
                *entry = IndexEntry {
                    key,
                    relation_id,
                    occupied: true,
                };
            }
            return true;
        }

        slot += 1;
    }

    false
}

unsafe fn index_remove(index: *mut IndexEntry, relation_id: u64) -> bool {
    let mut removed = false;
    let mut slot = 0usize;

    while slot < MAX_RELATIONS {
        let entry = unsafe { index.add(slot) };

        if unsafe { (*entry).occupied && (*entry).relation_id == relation_id } {
            unsafe {
                *entry = IndexEntry::empty();
            }
            removed = true;
        }

        slot += 1;
    }

    removed
}

unsafe fn index_find(index: *const IndexEntry, key: u64) -> Option<u64> {
    let mut slot = 0usize;

    while slot < MAX_RELATIONS {
        let entry = unsafe { index.add(slot) };

        if unsafe { (*entry).occupied && (*entry).key == key } {
            return Some(unsafe { (*entry).relation_id });
        }

        slot += 1;
    }

    None
}

fn reset_registry() {
    unsafe {
        let mut index = 0usize;

        while index < MAX_RELATIONS {
            RELATIONS[index] = RelationEntry::empty();
            SOURCE_INDEX[index] = IndexEntry::empty();
            TARGET_INDEX[index] = IndexEntry::empty();
            SEMANTIC_INDEX[index] = IndexEntry::empty();
            index += 1;
        }

        // Reset is private and exists only so self_test() can restore its
        // isolated test environment. Production relation IDs are never
        // reset or reused by add/remove.
        NEXT_ID = 1;
    }
}

fn next_relation_id() -> Option<u64> {
    unsafe {
        if NEXT_ID == 0 {
            return None;
        }

        let id = NEXT_ID;

        NEXT_ID = match NEXT_ID.checked_add(1) {
            Some(next_id) => next_id,
            None => 0,
        };

        Some(id)
    }
}

pub fn count() -> usize {
    let mut total = 0usize;

    unsafe {
        let mut index = 0usize;

        while index < MAX_RELATIONS {
            if RELATIONS[index].occupied {
                total += 1;
            }

            index += 1;
        }
    }

    total
}

pub fn contains(id: u64) -> bool {
    if id == 0 {
        return false;
    }

    unsafe {
        let mut index = 0usize;

        while index < MAX_RELATIONS {
            if RELATIONS[index].occupied && RELATIONS[index].id == id {
                return true;
            }

            index += 1;
        }
    }

    false
}

pub fn add(relation: Relation) -> Option<u64> {
    if !is_valid(&relation) {
        return None;
    }

    let metadata = default_metadata();

    if !metadata_is_valid(&metadata) {
        return None;
    }

    unsafe {
        let mut relation_slot = 0usize;

        while relation_slot < MAX_RELATIONS {
            if !RELATIONS[relation_slot].occupied {
                let id = next_relation_id()?;

                if !index_insert(
                    &raw mut SOURCE_INDEX as *mut IndexEntry,
                    relation.source,
                    id,
                ) {
                    return None;
                }

                if !index_insert(
                    &raw mut TARGET_INDEX as *mut IndexEntry,
                    relation.target,
                    id,
                ) {
                    index_remove(&raw mut SOURCE_INDEX as *mut IndexEntry, id);
                    return None;
                }

                if !index_insert(
                    &raw mut SEMANTIC_INDEX as *mut IndexEntry,
                    relation.relation as u64,
                    id,
                ) {
                    index_remove(&raw mut SOURCE_INDEX as *mut IndexEntry, id);
                    index_remove(&raw mut TARGET_INDEX as *mut IndexEntry, id);
                    return None;
                }

                let provenance = default_provenance();
                let integrity = integrity_hash(id, &relation, &metadata, &provenance);
                RELATIONS[relation_slot] = RelationEntry {
                    id,
                    relation,
                    metadata,
                    provenance,
                    trust: RelationTrust::Unverified,
                    integrity,
                    occupied: true,
                };

                return Some(id);
            }

            relation_slot += 1;
        }
    }

    None
}

/// Visits every occupied relation in deterministic registry-slot order.
///
/// The callback receives a copied relation ID and relation record, so the
/// registry never exposes mutable storage to graph consumers.
pub fn for_each<F: FnMut(u64, &Relation)>(mut callback: F) {
    unsafe {
        let mut index = 0usize;
        while index < MAX_RELATIONS {
            if RELATIONS[index].occupied {
                let id = RELATIONS[index].id;
                let relation = RELATIONS[index].relation;
                callback(id, &relation);
            }
            index += 1;
        }
    }
}

pub fn with_relation<R>(id: u64, callback: impl FnOnce(&Relation) -> R) -> Option<R> {
    if id == 0 {
        return None;
    }

    unsafe {
        let mut index = 0usize;

        while index < MAX_RELATIONS {
            if RELATIONS[index].occupied && RELATIONS[index].id == id {
                let relation = RELATIONS[index].relation;
                return Some(callback(&relation));
            }

            index += 1;
        }
    }

    None
}

pub fn with_metadata<R>(id: u64, callback: impl FnOnce(&RelationMetadata) -> R) -> Option<R> {
    if id == 0 {
        return None;
    }

    unsafe {
        let mut index = 0usize;

        while index < MAX_RELATIONS {
            if RELATIONS[index].occupied && RELATIONS[index].id == id {
                let metadata = RELATIONS[index].metadata;
                return Some(callback(&metadata));
            }

            index += 1;
        }
    }

    None
}

/// Reads immutable provenance for a relation without exposing registry storage.
pub fn with_provenance<R>(id: u64, callback: impl FnOnce(&RelationProvenance) -> R) -> Option<R> {
    if id == 0 {
        return None;
    }
    unsafe {
        let mut index = 0usize;
        while index < MAX_RELATIONS {
            if RELATIONS[index].occupied && RELATIONS[index].id == id {
                let provenance = RELATIONS[index].provenance;
                return Some(callback(&provenance));
            }
            index += 1;
        }
    }
    None
}

/// Reads the current trust state for a relation.
pub fn trust(id: u64) -> Option<RelationTrust> {
    if id == 0 {
        return None;
    }
    unsafe {
        let mut index = 0usize;
        while index < MAX_RELATIONS {
            if RELATIONS[index].occupied && RELATIONS[index].id == id {
                return Some(RELATIONS[index].trust);
            }
            index += 1;
        }
    }
    None
}

/// Returns the stored integrity value for a relation.
pub fn integrity(id: u64) -> Option<u64> {
    if id == 0 {
        return None;
    }
    unsafe {
        let mut index = 0usize;
        while index < MAX_RELATIONS {
            if RELATIONS[index].occupied && RELATIONS[index].id == id {
                return Some(RELATIONS[index].integrity);
            }
            index += 1;
        }
    }
    None
}

/// Recomputes and verifies a relation's deterministic integrity value.
pub fn verify_integrity(id: u64) -> bool {
    if id == 0 {
        return false;
    }
    unsafe {
        let mut index = 0usize;
        while index < MAX_RELATIONS {
            if RELATIONS[index].occupied && RELATIONS[index].id == id {
                let valid = relation_entry_integrity_valid(&RELATIONS[index]);
                RELATIONS[index].trust = if valid {
                    RelationTrust::Verified
                } else {
                    RelationTrust::Invalid
                };
                return valid;
            }
            index += 1;
        }
    }
    false
}

/// Marks a relation as conflicted without changing its data.
pub fn mark_conflict(id: u64) -> bool {
    if id == 0 {
        return false;
    }
    unsafe {
        let mut index = 0usize;
        while index < MAX_RELATIONS {
            if RELATIONS[index].occupied && RELATIONS[index].id == id {
                RELATIONS[index].trust = RelationTrust::Conflict;
                return true;
            }
            index += 1;
        }
    }
    false
}

/// Returns whether an incoming update is newer than the stored relation version.
pub fn is_newer(id: u64, incoming_version: u16, incoming_revision: u64) -> Option<bool> {
    if id == 0 || incoming_version == 0 || incoming_revision == 0 {
        return None;
    }
    unsafe {
        let mut index = 0usize;
        while index < MAX_RELATIONS {
            if RELATIONS[index].occupied && RELATIONS[index].id == id {
                let current = RELATIONS[index];
                return Some(
                    incoming_version > current.metadata.version
                        || (incoming_version == current.metadata.version
                            && incoming_revision > current.provenance.revision),
                );
            }
            index += 1;
        }
    }
    None
}

pub fn find(
    source: ObjectId,
    relation: SemanticId,
    target: ObjectId,
    context: ContextId,
) -> Option<u64> {
    unsafe {
        let mut index = 0usize;

        while index < MAX_RELATIONS {
            if RELATIONS[index].occupied {
                let candidate = RELATIONS[index].relation;

                if candidate.source == source
                    && candidate.relation == relation
                    && candidate.target == target
                    && candidate.context == context
                {
                    return Some(RELATIONS[index].id);
                }
            }

            index += 1;
        }
    }

    None
}

/// Returns the first relation ID indexed by source object.
pub fn find_by_source(source: ObjectId) -> Option<u64> {
    if source == 0 {
        return None;
    }

    unsafe { index_find(&raw const SOURCE_INDEX as *const IndexEntry, source) }
}

/// Returns the first relation ID indexed by target object.
pub fn find_by_target(target: ObjectId) -> Option<u64> {
    if target == 0 {
        return None;
    }

    unsafe { index_find(&raw const TARGET_INDEX as *const IndexEntry, target) }
}

/// Returns the first relation ID indexed by semantic relation type.
pub fn find_by_semantic(relation: SemanticId) -> Option<u64> {
    if !super::semantic::is_valid(relation) {
        return None;
    }

    unsafe {
        index_find(
            &raw const SEMANTIC_INDEX as *const IndexEntry,
            relation as u64,
        )
    }
}

pub fn remove(id: u64) -> bool {
    if id == 0 {
        return false;
    }

    unsafe {
        let mut index = 0usize;

        while index < MAX_RELATIONS {
            if RELATIONS[index].occupied && RELATIONS[index].id == id {
                RELATIONS[index] = RelationEntry::empty();

                index_remove(&raw mut SOURCE_INDEX as *mut IndexEntry, id);
                index_remove(&raw mut TARGET_INDEX as *mut IndexEntry, id);
                index_remove(&raw mut SEMANTIC_INDEX as *mut IndexEntry, id);

                return true;
            }

            index += 1;
        }
    }

    false
}

/// Maximum number of distinct graph nodes that can be encountered while
/// traversing a registry containing `MAX_RELATIONS` edges.
pub const MAX_TRAVERSAL_NODES: usize = MAX_RELATIONS + 1;

/// Maximum serialized bytes required for the complete relation registry.
pub const SERIALIZATION_MAGIC: u32 = 0x5447_524C; // "TGRL"
pub const SERIALIZATION_VERSION: u16 = 2;
pub const LEGACY_SERIALIZATION_VERSION: u16 = 1;
pub const SERIALIZED_HEADER_BYTES: usize = 8;
pub const LEGACY_SERIALIZED_RELATION_BYTES: usize = 58;
pub const SERIALIZED_RELATION_BYTES: usize = 76;
pub const MAX_SERIALIZED_BYTES: usize =
    SERIALIZED_HEADER_BYTES + (MAX_RELATIONS * SERIALIZED_RELATION_BYTES);
pub const SYNC_FRAME_BYTES: usize = 77;

/// Network synchronization operation for a relation record.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SyncOperation {
    Add = 1,
    Remove = 2,
    Update = 3,
}

/// Fixed-size, transport-neutral relation synchronization frame.
///
/// This deliberately stops at the GNOSIS relation boundary: a transport such
/// as the existing messaging/mesh layer can carry these bytes without making
/// the relation engine depend on a particular network implementation.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct RelationSyncFrame {
    pub operation: SyncOperation,
    pub relation_id: u64,
    pub relation: Relation,
    pub metadata: RelationMetadata,
    pub provenance: RelationProvenance,
    pub trust: RelationTrust,
    pub integrity: u64,
}

/// A bounded recursive graph query.
///
/// `None` for `relation` or `context` means that field is unconstrained.
/// `Some(0)` for `context` is therefore a valid exact context query.
#[derive(Clone, Copy)]
pub struct RecursiveQuery {
    pub source: ObjectId,
    pub target: ObjectId,
    pub relation: Option<SemanticId>,
    pub context: Option<ContextId>,
    pub max_depth: usize,
}

/// Fixed-capacity path returned by the pathfinding engine.
#[derive(Clone, Copy)]
pub struct RelationPath {
    pub nodes: [ObjectId; MAX_TRAVERSAL_NODES],
    pub relation_ids: [u64; MAX_RELATIONS],
    pub node_count: usize,
    pub relation_count: usize,
}

impl RelationPath {
    const fn empty() -> Self {
        Self {
            nodes: [0; MAX_TRAVERSAL_NODES],
            relation_ids: [0; MAX_RELATIONS],
            node_count: 0,
            relation_count: 0,
        }
    }

    pub fn is_valid(&self) -> bool {
        if self.node_count == 0
            || self.node_count > MAX_TRAVERSAL_NODES
            || self.relation_count + 1 != self.node_count
            || self.relation_count > MAX_RELATIONS
        {
            return false;
        }

        let mut index = 0usize;
        while index < self.node_count {
            if self.nodes[index] == 0 {
                return false;
            }
            index += 1;
        }

        let mut relation_index = 0usize;
        while relation_index < self.relation_count {
            if self.relation_ids[relation_index] == 0 {
                return false;
            }
            relation_index += 1;
        }

        true
    }

    pub fn depth(&self) -> usize {
        self.relation_count
    }

    pub fn node(&self, index: usize) -> Option<ObjectId> {
        if index < self.node_count {
            Some(self.nodes[index])
        } else {
            None
        }
    }

    pub fn relation_id(&self, index: usize) -> Option<u64> {
        if index < self.relation_count {
            Some(self.relation_ids[index])
        } else {
            None
        }
    }
}

fn query_matches(relation: &Relation, query: &RecursiveQuery) -> bool {
    if let Some(semantic) = query.relation {
        if relation.relation != semantic {
            return false;
        }
    }

    if let Some(context) = query.context {
        if relation.context != context {
            return false;
        }
    }

    true
}

/// Returns the shortest matching depth for a bounded recursive query.
///
/// The implementation is iterative BFS rather than CPU-stack recursion. This
/// keeps the kernel execution bounded and deterministic while providing the
/// recursive multi-hop semantics required by GNOSIS queries.
pub fn recursive_query(query: RecursiveQuery) -> Option<usize> {
    if query.source == 0 || query.target == 0 {
        return None;
    }

    if let Some(relation) = query.relation {
        if !super::semantic::is_valid(relation) {
            return None;
        }
    }

    if query.source == query.target {
        return Some(0);
    }

    if query.max_depth == 0 || count() == 0 || find_by_source(query.source).is_none() {
        return None;
    }

    let mut queue = [0u64; MAX_TRAVERSAL_NODES];
    let mut depths = [0usize; MAX_TRAVERSAL_NODES];
    let mut visited = [0u64; MAX_TRAVERSAL_NODES];
    let mut queue_head = 0usize;
    let mut queue_tail = 1usize;
    let mut visited_count = 1usize;

    queue[0] = query.source;
    visited[0] = query.source;
    depths[0] = 0;

    while queue_head < queue_tail {
        let current = queue[queue_head];
        let current_depth = depths[queue_head];
        queue_head += 1;

        if current_depth >= query.max_depth {
            continue;
        }

        unsafe {
            let mut relation_index = 0usize;
            while relation_index < MAX_RELATIONS {
                let entry = &RELATIONS[relation_index];
                if entry.occupied
                    && entry.relation.source == current
                    && query_matches(&entry.relation, &query)
                {
                    let next = entry.relation.target;

                    if next == query.target {
                        return Some(current_depth + 1);
                    }

                    let mut seen = false;
                    let mut visited_index = 0usize;
                    while visited_index < visited_count {
                        if visited[visited_index] == next {
                            seen = true;
                            break;
                        }
                        visited_index += 1;
                    }

                    if !seen {
                        if visited_count >= MAX_TRAVERSAL_NODES || queue_tail >= MAX_TRAVERSAL_NODES
                        {
                            return None;
                        }

                        visited[visited_count] = next;
                        visited_count += 1;
                        queue[queue_tail] = next;
                        depths[queue_tail] = current_depth + 1;
                        queue_tail += 1;
                    }
                }
                relation_index += 1;
            }
        }
    }

    None
}

/// Returns whether `target` is reachable from `source` within `max_depth`.
pub fn reachable(source: ObjectId, target: ObjectId, max_depth: usize) -> bool {
    recursive_query(RecursiveQuery {
        source,
        target,
        relation: None,
        context: None,
        max_depth,
    })
    .is_some()
}

/// Finds the shortest fixed-capacity path matching the supplied query.
pub fn find_path(query: RecursiveQuery) -> Option<RelationPath> {
    if query.source == 0 || query.target == 0 {
        return None;
    }

    if let Some(relation) = query.relation {
        if !super::semantic::is_valid(relation) {
            return None;
        }
    }

    let mut path = RelationPath::empty();
    path.nodes[0] = query.source;
    path.node_count = 1;

    if query.source == query.target {
        return Some(path);
    }

    if query.max_depth == 0 || count() == 0 || find_by_source(query.source).is_none() {
        return None;
    }

    let mut queue = [0u64; MAX_TRAVERSAL_NODES];
    let mut depths = [0usize; MAX_TRAVERSAL_NODES];
    let mut visited = [0u64; MAX_TRAVERSAL_NODES];
    let mut parents = [usize::MAX; MAX_TRAVERSAL_NODES];
    let mut parent_relations = [0u64; MAX_TRAVERSAL_NODES];
    let mut queue_head = 0usize;
    let mut queue_tail = 1usize;
    let mut visited_count = 1usize;
    let mut target_index = usize::MAX;

    queue[0] = query.source;
    visited[0] = query.source;
    depths[0] = 0;

    'bfs: while queue_head < queue_tail {
        let current = queue[queue_head];
        let current_depth = depths[queue_head];
        queue_head += 1;

        if current_depth >= query.max_depth {
            continue;
        }

        unsafe {
            let mut relation_index = 0usize;
            while relation_index < MAX_RELATIONS {
                let entry = &RELATIONS[relation_index];
                if entry.occupied
                    && entry.relation.source == current
                    && query_matches(&entry.relation, &query)
                {
                    let next = entry.relation.target;
                    let mut seen_index = 0usize;
                    let mut next_index = usize::MAX;

                    while seen_index < visited_count {
                        if visited[seen_index] == next {
                            next_index = seen_index;
                            break;
                        }
                        seen_index += 1;
                    }

                    if next_index == usize::MAX {
                        if visited_count >= MAX_TRAVERSAL_NODES || queue_tail >= MAX_TRAVERSAL_NODES
                        {
                            return None;
                        }

                        next_index = visited_count;
                        visited[next_index] = next;
                        visited_count += 1;
                        queue[queue_tail] = next;
                        depths[queue_tail] = current_depth + 1;
                        queue_tail += 1;
                        parents[next_index] = queue_head - 1;
                        parent_relations[next_index] = entry.id;
                    }

                    if next == query.target {
                        target_index = next_index;
                        break 'bfs;
                    }
                }
                relation_index += 1;
            }
        }
    }

    if target_index == usize::MAX {
        return None;
    }

    let depth = depths[target_index];
    if depth > MAX_RELATIONS || depth > query.max_depth {
        return None;
    }

    let mut reverse_nodes = [0u64; MAX_TRAVERSAL_NODES];
    let mut reverse_relations = [0u64; MAX_RELATIONS];
    let mut reverse_count = 0usize;
    let mut current_index = target_index;

    reverse_nodes[reverse_count] = visited[current_index];
    reverse_count += 1;

    while current_index != 0 {
        if reverse_count >= MAX_TRAVERSAL_NODES {
            return None;
        }
        reverse_relations[reverse_count - 1] = parent_relations[current_index];
        current_index = parents[current_index];
        reverse_nodes[reverse_count] = visited[current_index];
        reverse_count += 1;
    }

    path.node_count = reverse_count;
    path.relation_count = reverse_count - 1;

    let mut index = 0usize;
    while index < reverse_count {
        path.nodes[index] = reverse_nodes[reverse_count - 1 - index];
        index += 1;
    }

    index = 0;
    while index < path.relation_count {
        path.relation_ids[index] = reverse_relations[path.relation_count - 1 - index];
        index += 1;
    }

    if path.is_valid() { Some(path) } else { None }
}

fn write_u16(buffer: &mut [u8], offset: &mut usize, value: u16) -> bool {
    if *offset + 2 > buffer.len() {
        return false;
    }
    let bytes = value.to_le_bytes();
    buffer[*offset] = bytes[0];
    buffer[*offset + 1] = bytes[1];
    *offset += 2;
    true
}

fn write_u32(buffer: &mut [u8], offset: &mut usize, value: u32) -> bool {
    if *offset + 4 > buffer.len() {
        return false;
    }
    let bytes = value.to_le_bytes();
    let mut index = 0usize;
    while index < 4 {
        buffer[*offset + index] = bytes[index];
        index += 1;
    }
    *offset += 4;
    true
}

fn write_u64(buffer: &mut [u8], offset: &mut usize, value: u64) -> bool {
    if *offset + 8 > buffer.len() {
        return false;
    }
    let bytes = value.to_le_bytes();
    let mut index = 0usize;
    while index < 8 {
        buffer[*offset + index] = bytes[index];
        index += 1;
    }
    *offset += 8;
    true
}

fn read_u16(data: &[u8], offset: &mut usize) -> Option<u16> {
    if *offset + 2 > data.len() {
        return None;
    }
    let value = u16::from_le_bytes([data[*offset], data[*offset + 1]]);
    *offset += 2;
    Some(value)
}

fn read_u32(data: &[u8], offset: &mut usize) -> Option<u32> {
    if *offset + 4 > data.len() {
        return None;
    }
    let value = u32::from_le_bytes([
        data[*offset],
        data[*offset + 1],
        data[*offset + 2],
        data[*offset + 3],
    ]);
    *offset += 4;
    Some(value)
}

fn read_u64(data: &[u8], offset: &mut usize) -> Option<u64> {
    if *offset + 8 > data.len() {
        return None;
    }
    let value = u64::from_le_bytes([
        data[*offset],
        data[*offset + 1],
        data[*offset + 2],
        data[*offset + 3],
        data[*offset + 4],
        data[*offset + 5],
        data[*offset + 6],
        data[*offset + 7],
    ]);
    *offset += 8;
    Some(value)
}

/// Serializes the complete active relation registry into a caller-provided buffer.
pub fn serialize(buffer: &mut [u8]) -> Option<usize> {
    let relation_count = count();
    if relation_count > MAX_RELATIONS
        || buffer.len() < SERIALIZED_HEADER_BYTES + relation_count * SERIALIZED_RELATION_BYTES
    {
        return None;
    }

    let mut offset = 0usize;
    if !write_u32(buffer, &mut offset, SERIALIZATION_MAGIC)
        || !write_u16(buffer, &mut offset, SERIALIZATION_VERSION)
        || !write_u16(buffer, &mut offset, relation_count as u16)
    {
        return None;
    }

    unsafe {
        let mut index = 0usize;
        while index < MAX_RELATIONS {
            if RELATIONS[index].occupied {
                let entry = RELATIONS[index];
                if !write_u64(buffer, &mut offset, entry.id)
                    || !write_u64(buffer, &mut offset, entry.relation.source)
                    || !write_u32(buffer, &mut offset, entry.relation.relation)
                    || !write_u64(buffer, &mut offset, entry.relation.target)
                    || !write_u64(buffer, &mut offset, entry.relation.context)
                    || !write_u64(buffer, &mut offset, entry.metadata.created_at)
                    || !write_u64(buffer, &mut offset, entry.metadata.updated_at)
                    || !write_u16(buffer, &mut offset, entry.metadata.version)
                    || !write_u32(buffer, &mut offset, entry.metadata.state)
                    || !write_u64(buffer, &mut offset, entry.provenance.origin)
                {
                    return None;
                }
                if offset + 1 > buffer.len() {
                    return None;
                }
                buffer[offset] = entry.provenance.source as u8;
                offset += 1;
                if !write_u64(buffer, &mut offset, entry.provenance.revision)
                    || offset + 1 > buffer.len()
                {
                    return None;
                }
                buffer[offset] = entry.trust as u8;
                offset += 1;
                if !write_u64(buffer, &mut offset, entry.integrity) {
                    return None;
                }
            }
            index += 1;
        }
    }
    Some(offset)
}

/// Restores a serialized relation registry atomically. Version 1 payloads from
/// REL-01.6 remain readable and are imported as recovered, unverified records.
pub fn deserialize(data: &[u8]) -> bool {
    if data.len() < SERIALIZED_HEADER_BYTES {
        return false;
    }
    let mut offset = 0usize;
    let magic = match read_u32(data, &mut offset) {
        Some(value) => value,
        None => return false,
    };
    let version = match read_u16(data, &mut offset) {
        Some(value) => value,
        None => return false,
    };
    let relation_count = match read_u16(data, &mut offset) {
        Some(value) => value as usize,
        None => return false,
    };
    if magic != SERIALIZATION_MAGIC || relation_count > MAX_RELATIONS {
        return false;
    }
    let record_bytes = match version {
        LEGACY_SERIALIZATION_VERSION => LEGACY_SERIALIZED_RELATION_BYTES,
        SERIALIZATION_VERSION => SERIALIZED_RELATION_BYTES,
        _ => return false,
    };
    let expected_len = SERIALIZED_HEADER_BYTES + relation_count * record_bytes;
    if data.len() != expected_len {
        return false;
    }

    let mut new_relations = [RelationEntry::empty(); MAX_RELATIONS];
    let mut new_source_index = [IndexEntry::empty(); MAX_RELATIONS];
    let mut new_target_index = [IndexEntry::empty(); MAX_RELATIONS];
    let mut new_semantic_index = [IndexEntry::empty(); MAX_RELATIONS];
    let mut highest_id = 0u64;

    let mut record = 0usize;
    while record < relation_count {
        let id = match read_u64(data, &mut offset) {
            Some(value) => value,
            None => return false,
        };
        let relation = Relation {
            source: match read_u64(data, &mut offset) {
                Some(value) => value,
                None => return false,
            },
            relation: match read_u32(data, &mut offset) {
                Some(value) => value,
                None => return false,
            },
            target: match read_u64(data, &mut offset) {
                Some(value) => value,
                None => return false,
            },
            context: match read_u64(data, &mut offset) {
                Some(value) => value,
                None => return false,
            },
        };
        let metadata = RelationMetadata {
            created_at: match read_u64(data, &mut offset) {
                Some(value) => value,
                None => return false,
            },
            updated_at: match read_u64(data, &mut offset) {
                Some(value) => value,
                None => return false,
            },
            version: match read_u16(data, &mut offset) {
                Some(value) => value,
                None => return false,
            },
            state: match read_u32(data, &mut offset) {
                Some(value) => value,
                None => return false,
            },
        };

        let (provenance, trust, stored_integrity) = if version == LEGACY_SERIALIZATION_VERSION {
            (
                RelationProvenance {
                    origin: 0,
                    source: ProvenanceSource::Recovered,
                    revision: 1,
                },
                RelationTrust::Unverified,
                0,
            )
        } else {
            let origin = match read_u64(data, &mut offset) {
                Some(value) => value,
                None => return false,
            };
            let source = match data.get(offset).copied() {
                Some(1) => ProvenanceSource::Local,
                Some(2) => ProvenanceSource::Remote,
                Some(3) => ProvenanceSource::Recovered,
                _ => return false,
            };
            offset += 1;
            let revision = match read_u64(data, &mut offset) {
                Some(value) => value,
                None => return false,
            };
            let trust = match data.get(offset).copied() {
                Some(1) => RelationTrust::Unverified,
                Some(2) => RelationTrust::Verified,
                Some(3) => RelationTrust::Conflict,
                Some(4) => RelationTrust::Invalid,
                _ => return false,
            };
            offset += 1;
            let integrity = match read_u64(data, &mut offset) {
                Some(value) => value,
                None => return false,
            };
            (
                RelationProvenance {
                    origin,
                    source,
                    revision,
                },
                trust,
                integrity,
            )
        };

        if id == 0
            || !is_valid(&relation)
            || !metadata_is_valid(&metadata)
            || !provenance_is_valid(&provenance)
            || !trust_is_valid(trust)
        {
            return false;
        }
        let mut slot = 0usize;
        while slot < record {
            if new_relations[slot].occupied && new_relations[slot].id == id {
                return false;
            }
            slot += 1;
        }
        let mut index = 0usize;
        while index < MAX_RELATIONS {
            if !new_relations[index].occupied {
                let computed_integrity = integrity_hash(id, &relation, &metadata, &provenance);
                let final_integrity = if version == LEGACY_SERIALIZATION_VERSION {
                    computed_integrity
                } else if stored_integrity != computed_integrity {
                    return false;
                } else {
                    stored_integrity
                };
                new_relations[index] = RelationEntry {
                    id,
                    relation,
                    metadata,
                    provenance,
                    trust,
                    integrity: final_integrity,
                    occupied: true,
                };
                break;
            }
            index += 1;
        }
        if index == MAX_RELATIONS {
            return false;
        }
        let indexes_ok = unsafe {
            index_insert(new_source_index.as_mut_ptr(), relation.source, id)
                && index_insert(new_target_index.as_mut_ptr(), relation.target, id)
                && index_insert(
                    new_semantic_index.as_mut_ptr(),
                    relation.relation as u64,
                    id,
                )
        };
        if !indexes_ok {
            return false;
        }
        if id > highest_id {
            highest_id = id;
        }
        record += 1;
    }

    let next_id = match highest_id.checked_add(1) {
        Some(value) if value != 0 => value,
        _ => 0,
    };
    unsafe {
        RELATIONS = new_relations;
        SOURCE_INDEX = new_source_index;
        TARGET_INDEX = new_target_index;
        SEMANTIC_INDEX = new_semantic_index;
        NEXT_ID = next_id;
    }
    true
}

/// Computes a deterministic 64-bit hash of the canonical serialized registry.
/// This is an integrity hash, not a cryptographic proof.
pub fn hash() -> Option<u64> {
    let mut buffer = [0u8; MAX_SERIALIZED_BYTES];
    let length = serialize(&mut buffer)?;
    let mut state = 0xcbf2_9ce4_8422_2325u64;
    let mut index = 0usize;
    while index < length {
        state ^= buffer[index] as u64;
        state = state.wrapping_mul(0x0000_0100_0000_01b3u64);
        index += 1;
    }
    Some(state)
}

/// Encodes one synchronization frame into a fixed wire representation.
pub fn encode_sync_frame(frame: &RelationSyncFrame, buffer: &mut [u8]) -> Option<usize> {
    if buffer.len() < SYNC_FRAME_BYTES
        || frame.relation_id == 0
        || !is_valid(&frame.relation)
        || !metadata_is_valid(&frame.metadata)
        || !provenance_is_valid(&frame.provenance)
        || !trust_is_valid(frame.trust)
    {
        return None;
    }
    let mut offset = 0usize;
    buffer[offset] = frame.operation as u8;
    offset += 1;
    if !write_u64(buffer, &mut offset, frame.relation_id)
        || !write_u64(buffer, &mut offset, frame.relation.source)
        || !write_u32(buffer, &mut offset, frame.relation.relation)
        || !write_u64(buffer, &mut offset, frame.relation.target)
        || !write_u64(buffer, &mut offset, frame.relation.context)
        || !write_u64(buffer, &mut offset, frame.metadata.created_at)
        || !write_u64(buffer, &mut offset, frame.metadata.updated_at)
        || !write_u16(buffer, &mut offset, frame.metadata.version)
        || !write_u32(buffer, &mut offset, frame.metadata.state)
        || !write_u64(buffer, &mut offset, frame.provenance.origin)
    {
        return None;
    }
    if offset + 1 > buffer.len() {
        return None;
    }
    buffer[offset] = frame.provenance.source as u8;
    offset += 1;
    if !write_u64(buffer, &mut offset, frame.provenance.revision) || offset + 1 > buffer.len() {
        return None;
    }
    buffer[offset] = frame.trust as u8;
    offset += 1;
    if !write_u64(buffer, &mut offset, frame.integrity) {
        return None;
    }
    Some(offset)
}

/// Decodes a fixed synchronization frame without allocating memory.
pub fn decode_sync_frame(data: &[u8]) -> Option<RelationSyncFrame> {
    if data.len() != SYNC_FRAME_BYTES {
        return None;
    }
    let operation = match data[0] {
        1 => SyncOperation::Add,
        2 => SyncOperation::Remove,
        3 => SyncOperation::Update,
        _ => return None,
    };
    let mut offset = 1usize;
    let relation_id = read_u64(data, &mut offset)?;
    let relation = Relation {
        source: read_u64(data, &mut offset)?,
        relation: read_u32(data, &mut offset)?,
        target: read_u64(data, &mut offset)?,
        context: read_u64(data, &mut offset)?,
    };
    let metadata = RelationMetadata {
        created_at: read_u64(data, &mut offset)?,
        updated_at: read_u64(data, &mut offset)?,
        version: read_u16(data, &mut offset)?,
        state: read_u32(data, &mut offset)?,
    };
    let provenance = RelationProvenance {
        origin: read_u64(data, &mut offset)?,
        source: match data.get(offset).copied() {
            Some(1) => ProvenanceSource::Local,
            Some(2) => ProvenanceSource::Remote,
            Some(3) => ProvenanceSource::Recovered,
            _ => return None,
        },
        revision: {
            offset += 1;
            read_u64(data, &mut offset)?
        },
    };
    let trust = match data.get(offset).copied() {
        Some(1) => RelationTrust::Unverified,
        Some(2) => RelationTrust::Verified,
        Some(3) => RelationTrust::Conflict,
        Some(4) => RelationTrust::Invalid,
        _ => return None,
    };
    offset += 1;
    let integrity = read_u64(data, &mut offset)?;
    if relation_id == 0
        || !is_valid(&relation)
        || !metadata_is_valid(&metadata)
        || !provenance_is_valid(&provenance)
        || !trust_is_valid(trust)
        || integrity_hash(relation_id, &relation, &metadata, &provenance) != integrity
    {
        return None;
    }
    Some(RelationSyncFrame {
        operation,
        relation_id,
        relation,
        metadata,
        provenance,
        trust,
        integrity,
    })
}

/// Applies a synchronization frame using deterministic version/revision rules.
pub fn apply_sync_frame(frame: RelationSyncFrame) -> bool {
    if frame.relation_id == 0
        || !is_valid(&frame.relation)
        || !metadata_is_valid(&frame.metadata)
        || !provenance_is_valid(&frame.provenance)
        || !trust_is_valid(frame.trust)
        || integrity_hash(
            frame.relation_id,
            &frame.relation,
            &frame.metadata,
            &frame.provenance,
        ) != frame.integrity
    {
        return false;
    }

    match frame.operation {
        SyncOperation::Remove => remove(frame.relation_id),
        SyncOperation::Add | SyncOperation::Update => unsafe {
            let mut slot = 0usize;
            while slot < MAX_RELATIONS {
                if RELATIONS[slot].occupied && RELATIONS[slot].id == frame.relation_id {
                    let existing = RELATIONS[slot];
                    let same = existing.relation.source == frame.relation.source
                        && existing.relation.relation == frame.relation.relation
                        && existing.relation.target == frame.relation.target
                        && existing.relation.context == frame.relation.context
                        && existing.metadata.created_at == frame.metadata.created_at
                        && existing.metadata.updated_at == frame.metadata.updated_at
                        && existing.metadata.version == frame.metadata.version
                        && existing.metadata.state == frame.metadata.state
                        && existing.provenance.origin == frame.provenance.origin
                        && existing.provenance.source == frame.provenance.source
                        && existing.provenance.revision == frame.provenance.revision
                        && existing.integrity == frame.integrity;
                    if same {
                        return true;
                    }
                    let newer = frame.metadata.version > existing.metadata.version
                        || (frame.metadata.version == existing.metadata.version
                            && frame.provenance.revision > existing.provenance.revision);
                    if !newer {
                        RELATIONS[slot].trust = RelationTrust::Conflict;
                        return false;
                    }
                    index_remove(&raw mut SOURCE_INDEX as *mut IndexEntry, frame.relation_id);
                    index_remove(&raw mut TARGET_INDEX as *mut IndexEntry, frame.relation_id);
                    index_remove(
                        &raw mut SEMANTIC_INDEX as *mut IndexEntry,
                        frame.relation_id,
                    );
                    if !index_insert(
                        &raw mut SOURCE_INDEX as *mut IndexEntry,
                        frame.relation.source,
                        frame.relation_id,
                    ) || !index_insert(
                        &raw mut TARGET_INDEX as *mut IndexEntry,
                        frame.relation.target,
                        frame.relation_id,
                    ) || !index_insert(
                        &raw mut SEMANTIC_INDEX as *mut IndexEntry,
                        frame.relation.relation as u64,
                        frame.relation_id,
                    ) {
                        return false;
                    }
                    RELATIONS[slot] = RelationEntry {
                        id: frame.relation_id,
                        relation: frame.relation,
                        metadata: frame.metadata,
                        provenance: frame.provenance,
                        trust: frame.trust,
                        integrity: frame.integrity,
                        occupied: true,
                    };
                    return true;
                }
                slot += 1;
            }
            if frame.operation == SyncOperation::Update {
                return false;
            }
            let mut slot = 0usize;
            while slot < MAX_RELATIONS {
                if !RELATIONS[slot].occupied {
                    if !index_insert(
                        &raw mut SOURCE_INDEX as *mut IndexEntry,
                        frame.relation.source,
                        frame.relation_id,
                    ) || !index_insert(
                        &raw mut TARGET_INDEX as *mut IndexEntry,
                        frame.relation.target,
                        frame.relation_id,
                    ) || !index_insert(
                        &raw mut SEMANTIC_INDEX as *mut IndexEntry,
                        frame.relation.relation as u64,
                        frame.relation_id,
                    ) {
                        index_remove(&raw mut SOURCE_INDEX as *mut IndexEntry, frame.relation_id);
                        index_remove(&raw mut TARGET_INDEX as *mut IndexEntry, frame.relation_id);
                        index_remove(
                            &raw mut SEMANTIC_INDEX as *mut IndexEntry,
                            frame.relation_id,
                        );
                        return false;
                    }
                    RELATIONS[slot] = RelationEntry {
                        id: frame.relation_id,
                        relation: frame.relation,
                        metadata: frame.metadata,
                        provenance: frame.provenance,
                        trust: frame.trust,
                        integrity: frame.integrity,
                        occupied: true,
                    };
                    if frame.relation_id >= NEXT_ID {
                        NEXT_ID = match frame.relation_id.checked_add(1) {
                            Some(next) if next != 0 => next,
                            _ => 0,
                        };
                    }
                    return true;
                }
                slot += 1;
            }
            false
        },
    }
}

/// Creates a synchronization frame from an existing relation.
pub fn sync_frame(id: u64) -> Option<RelationSyncFrame> {
    if id == 0 {
        return None;
    }
    unsafe {
        let mut index = 0usize;
        while index < MAX_RELATIONS {
            if RELATIONS[index].occupied && RELATIONS[index].id == id {
                return Some(RelationSyncFrame {
                    operation: SyncOperation::Add,
                    relation_id: id,
                    relation: RELATIONS[index].relation,
                    metadata: RELATIONS[index].metadata,
                    provenance: RELATIONS[index].provenance,
                    trust: RELATIONS[index].trust,
                    integrity: RELATIONS[index].integrity,
                });
            }
            index += 1;
        }
    }
    None
}

pub fn self_test() -> bool {
    reset_registry();

    if count() != 0 || contains(0) {
        reset_registry();
        return false;
    }

    let valid_relation = Relation {
        source: 42,
        relation: super::semantic::CONNECTED_TO,
        target: 91,
        context: 7,
    };

    let id1 = match add(valid_relation) {
        Some(id) => id,
        None => {
            reset_registry();
            return false;
        }
    };

    if id1 != 1 || count() != 1 || !contains(id1) {
        reset_registry();
        return false;
    }

    let mut relation_ok = false;

    let _ = with_relation(id1, |stored| {
        relation_ok = stored.source == valid_relation.source
            && stored.relation == valid_relation.relation
            && stored.target == valid_relation.target
            && stored.context == valid_relation.context;
    });

    if !relation_ok || with_relation(0, |_| true).is_some() {
        reset_registry();
        return false;
    }

    if find(
        valid_relation.source,
        valid_relation.relation,
        valid_relation.target,
        valid_relation.context,
    ) != Some(id1)
    {
        reset_registry();
        return false;
    }

    let mut metadata_ok = false;

    let _ = with_metadata(id1, |metadata| {
        metadata_ok = metadata.version == CURRENT_VERSION
            && metadata.created_at == metadata.updated_at
            && metadata.state == super::semantic::ACTIVE;
    });

    if !metadata_ok || with_metadata(0, |_| true).is_some() {
        reset_registry();
        return false;
    }

    if find_by_source(valid_relation.source) != Some(id1)
        || find_by_target(valid_relation.target) != Some(id1)
        || find_by_semantic(valid_relation.relation) != Some(id1)
    {
        reset_registry();
        return false;
    }

    let id2 = match add(valid_relation) {
        Some(id) => id,
        None => {
            reset_registry();
            return false;
        }
    };

    if id2 != id1 + 1 || count() != 2 {
        reset_registry();
        return false;
    }

    if find(
        valid_relation.source,
        valid_relation.relation,
        valid_relation.target,
        valid_relation.context,
    ) != Some(id1)
    {
        reset_registry();
        return false;
    }

    if find_by_source(valid_relation.source) != Some(id1)
        || find_by_target(valid_relation.target) != Some(id1)
        || find_by_semantic(valid_relation.relation) != Some(id1)
    {
        reset_registry();
        return false;
    }

    if !remove(id1) || contains(id1) || !contains(id2) || count() != 1 {
        reset_registry();
        return false;
    }

    if find(
        valid_relation.source,
        valid_relation.relation,
        valid_relation.target,
        valid_relation.context,
    ) != Some(id2)
    {
        reset_registry();
        return false;
    }

    if find_by_source(valid_relation.source) != Some(id2)
        || find_by_target(valid_relation.target) != Some(id2)
        || find_by_semantic(valid_relation.relation) != Some(id2)
    {
        reset_registry();
        return false;
    }

    if find_by_source(0).is_some() || find_by_target(0).is_some() || find_by_semantic(0).is_some() {
        reset_registry();
        return false;
    }

    let id3 = match add(valid_relation) {
        Some(id) => id,
        None => {
            reset_registry();
            return false;
        }
    };

    if id3 != id2 + 1 || id3 == id1 || count() != 2 {
        reset_registry();
        return false;
    }

    let bad_source = Relation {
        source: 0,
        ..valid_relation
    };

    if add(bad_source).is_some() {
        reset_registry();
        return false;
    }

    let bad_target = Relation {
        target: 0,
        ..valid_relation
    };

    if add(bad_target).is_some() {
        reset_registry();
        return false;
    }

    let bad_semantic = Relation {
        relation: 0,
        ..valid_relation
    };

    if add(bad_semantic).is_some() {
        reset_registry();
        return false;
    }

    let context_zero = Relation {
        context: 0,
        ..valid_relation
    };

    let context_zero_id = match add(context_zero) {
        Some(id) => id,
        None => {
            reset_registry();
            return false;
        }
    };

    let self_relation = Relation {
        source: 5,
        target: 5,
        ..valid_relation
    };

    let self_relation_id = match add(self_relation) {
        Some(id) => id,
        None => {
            reset_registry();
            return false;
        }
    };

    if !contains(context_zero_id) || !contains(self_relation_id) || count() != 4 {
        reset_registry();
        return false;
    }

    if find_by_source(5) != Some(self_relation_id) || find_by_target(5) != Some(self_relation_id) {
        reset_registry();
        return false;
    }

    let occupied_before_fill = count();
    let mut fill_index = 0u64;

    while count() < CAPACITY {
        let unique_relation = Relation {
            source: 1_000 + fill_index,
            relation: super::semantic::CONNECTED_TO,
            target: 2_000 + fill_index,
            context: 0,
        };

        if add(unique_relation).is_none() {
            reset_registry();
            return false;
        }

        fill_index += 1;
    }

    if count() != CAPACITY
        || occupied_before_fill >= CAPACITY
        || fill_index as usize != CAPACITY - occupied_before_fill
    {
        reset_registry();
        return false;
    }

    if find_by_source(1_000).is_none()
        || find_by_target(2_000).is_none()
        || find_by_semantic(super::semantic::CONNECTED_TO).is_none()
    {
        reset_registry();
        return false;
    }

    let overflow_relation = Relation {
        source: 9_999,
        relation: super::semantic::CONNECTED_TO,
        target: 9_998,
        context: 0,
    };

    if add(overflow_relation).is_some() {
        reset_registry();
        return false;
    }

    reset_registry();

    if count() != 0
        || contains(id1)
        || contains(id2)
        || contains(id3)
        || contains(context_zero_id)
        || contains(self_relation_id)
        || find_by_source(valid_relation.source).is_some()
        || find_by_target(valid_relation.target).is_some()
        || find_by_semantic(valid_relation.relation).is_some()
    {
        return false;
    }

    let reset_id = match add(valid_relation) {
        Some(id) => id,
        None => return false,
    };

    let reset_ok = reset_id == 1 && count() == 1;

    reset_registry();

    if !reset_ok || count() != 0 {
        return false;
    }

    // REL-01.4 traversal, REL-01.5 recursive queries, REL-01.6 persistence/sync, and REL-01.7 integrity.
    let r12 = Relation {
        source: 1,
        relation: super::semantic::CONNECTED_TO,
        target: 2,
        context: 7,
    };
    let r23 = Relation {
        source: 2,
        relation: super::semantic::CONNECTED_TO,
        target: 3,
        context: 7,
    };
    let r34 = Relation {
        source: 3,
        relation: super::semantic::CAUSES,
        target: 4,
        context: 8,
    };
    let r45 = Relation {
        source: 4,
        relation: super::semantic::CONNECTED_TO,
        target: 5,
        context: 7,
    };

    let p12 = match add(r12) {
        Some(id) => id,
        None => return false,
    };
    let p23 = match add(r23) {
        Some(id) => id,
        None => {
            reset_registry();
            return false;
        }
    };
    let p34 = match add(r34) {
        Some(id) => id,
        None => {
            reset_registry();
            return false;
        }
    };
    let p45 = match add(r45) {
        Some(id) => id,
        None => {
            reset_registry();
            return false;
        }
    };

    if !reachable(1, 5, 4) || reachable(1, 5, 3) || !reachable(1, 1, 0) {
        reset_registry();
        return false;
    }

    let query_all = RecursiveQuery {
        source: 1,
        target: 5,
        relation: None,
        context: None,
        max_depth: 4,
    };
    if recursive_query(query_all) != Some(4) {
        reset_registry();
        return false;
    }

    let query_context = RecursiveQuery {
        source: 1,
        target: 3,
        relation: Some(super::semantic::CONNECTED_TO),
        context: Some(7),
        max_depth: 2,
    };
    if recursive_query(query_context) != Some(2) {
        reset_registry();
        return false;
    }

    let query_wrong_context = RecursiveQuery {
        source: 1,
        target: 5,
        relation: None,
        context: Some(8),
        max_depth: 4,
    };
    if recursive_query(query_wrong_context).is_some() {
        reset_registry();
        return false;
    }

    let path = match find_path(query_all) {
        Some(path) => path,
        None => {
            reset_registry();
            return false;
        }
    };
    if !path.is_valid()
        || path.depth() != 4
        || path.node_count != 5
        || path.nodes[0] != 1
        || path.nodes[4] != 5
        || path.relation_ids[0] != p12
        || path.relation_ids[1] != p23
        || path.relation_ids[2] != p34
        || path.relation_ids[3] != p45
    {
        reset_registry();
        return false;
    }

    let same_path = match find_path(RecursiveQuery {
        source: 3,
        target: 3,
        relation: None,
        context: None,
        max_depth: 0,
    }) {
        Some(path) => path,
        None => {
            reset_registry();
            return false;
        }
    };
    if !same_path.is_valid() || same_path.node_count != 1 || same_path.relation_count != 0 {
        reset_registry();
        return false;
    }

    let mut serialized = [0u8; MAX_SERIALIZED_BYTES];
    let serialized_len = match serialize(&mut serialized) {
        Some(length) => length,
        None => {
            reset_registry();
            return false;
        }
    };
    if serialized_len != SERIALIZED_HEADER_BYTES + count() * SERIALIZED_RELATION_BYTES {
        reset_registry();
        return false;
    }

    let registry_hash = match hash() {
        Some(value) => value,
        None => {
            reset_registry();
            return false;
        }
    };

    reset_registry();
    if count() != 0 || deserialize(&serialized[..serialized_len]) == false || count() != 4 {
        reset_registry();
        return false;
    }

    if hash() != Some(registry_hash)
        || !contains(p12)
        || !contains(p23)
        || !contains(p34)
        || !contains(p45)
    {
        reset_registry();
        return false;
    }

    let frame = match sync_frame(p12) {
        Some(frame) => frame,
        None => {
            reset_registry();
            return false;
        }
    };
    if frame.relation_id != p12 || frame.operation as u8 != SyncOperation::Add as u8 {
        reset_registry();
        return false;
    }

    let mut sync_bytes = [0u8; SYNC_FRAME_BYTES];
    let sync_len = match encode_sync_frame(&frame, &mut sync_bytes) {
        Some(length) => length,
        None => {
            reset_registry();
            return false;
        }
    };
    let decoded_frame = match decode_sync_frame(&sync_bytes[..sync_len]) {
        Some(decoded) => decoded,
        None => {
            reset_registry();
            return false;
        }
    };
    if decoded_frame.relation_id != frame.relation_id
        || decoded_frame.relation.source != frame.relation.source
        || decoded_frame.relation.target != frame.relation.target
        || decoded_frame.metadata.version != frame.metadata.version
    {
        reset_registry();
        return false;
    }

    if !remove(p12) || apply_sync_frame(decoded_frame) == false || !contains(p12) {
        reset_registry();
        return false;
    }

    // REL-01.7 provenance, integrity, trust, and deterministic version/conflict handling.
    if !verify_integrity(p12) || trust(p12) != Some(RelationTrust::Verified) {
        reset_registry();
        return false;
    }
    let provenance_ok = with_provenance(p12, |value| {
        value.source == ProvenanceSource::Local && value.revision == 1
    })
    .unwrap_or(false);
    if !provenance_ok
        || integrity(p12).is_none()
        || is_newer(p12, CURRENT_VERSION, 1) != Some(false)
    {
        reset_registry();
        return false;
    }

    let base_frame = match sync_frame(p12) {
        Some(frame) => frame,
        None => {
            reset_registry();
            return false;
        }
    };
    let mut newer_frame = base_frame;
    newer_frame.operation = SyncOperation::Update;
    newer_frame.metadata.updated_at = newer_frame.metadata.updated_at.saturating_add(1);
    newer_frame.metadata.version = CURRENT_VERSION.saturating_add(1);
    newer_frame.provenance.source = ProvenanceSource::Remote;
    newer_frame.provenance.origin = 99;
    newer_frame.provenance.revision = 2;
    newer_frame.trust = RelationTrust::Verified;
    newer_frame.integrity = integrity_hash(
        newer_frame.relation_id,
        &newer_frame.relation,
        &newer_frame.metadata,
        &newer_frame.provenance,
    );
    if !apply_sync_frame(newer_frame) || trust(p12) != Some(RelationTrust::Verified) {
        reset_registry();
        return false;
    }

    let stale_frame = base_frame;
    if apply_sync_frame(stale_frame) || trust(p12) != Some(RelationTrust::Conflict) {
        reset_registry();
        return false;
    }

    reset_registry();
    true
}
