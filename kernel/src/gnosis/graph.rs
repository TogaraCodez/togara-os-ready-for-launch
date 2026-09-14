#[allow(clippy::missing_safety_doc)]
// GNOSIS Knowledge Graph Engine — bounded graph operations over the relation engine.
//
// REL-01.8 adds graph-level views without changing the canonical relation model.
// The engine is fixed-capacity, deterministic, heap-free, and no-std compatible.
use super::relation;
use super::semantic::{ContextId, ObjectId, SemanticId};

pub const MAX_GRAPH_NODES: usize = relation::MAX_RELATIONS * 2;
pub const MAX_GRAPH_RELATIONS: usize = relation::MAX_RELATIONS;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GraphDirection {
    Outgoing = 1,
    Incoming = 2,
    Both = 3,
}

#[derive(Clone, Copy)]
pub struct GraphPattern {
    pub source: Option<ObjectId>,
    pub relation: Option<SemanticId>,
    pub target: Option<ObjectId>,
    pub context: Option<ContextId>,
}

#[derive(Clone, Copy)]
pub struct GraphSnapshot {
    pub nodes: [ObjectId; MAX_GRAPH_NODES],
    pub relation_ids: [u64; MAX_GRAPH_RELATIONS],
    pub node_count: usize,
    pub relation_count: usize,
}

impl GraphSnapshot {
    const fn empty() -> Self {
        Self {
            nodes: [0; MAX_GRAPH_NODES],
            relation_ids: [0; MAX_GRAPH_RELATIONS],
            node_count: 0,
            relation_count: 0,
        }
    }

    pub fn is_valid(&self) -> bool {
        if self.node_count == 0
            || self.node_count > MAX_GRAPH_NODES
            || self.relation_count > MAX_GRAPH_RELATIONS
        {
            return false;
        }

        let mut i = 0usize;
        while i < self.node_count {
            if self.nodes[i] == 0 {
                return false;
            }
            let mut j = i + 1;
            while j < self.node_count {
                if self.nodes[i] == self.nodes[j] {
                    return false;
                }
                j += 1;
            }
            i += 1;
        }

        let mut r = 0usize;
        while r < self.relation_count {
            if self.relation_ids[r] == 0 {
                return false;
            }
            r += 1;
        }
        true
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

#[derive(Clone, Copy)]
pub struct GraphStats {
    pub relation_count: usize,
    pub node_count: usize,
    pub source_node_count: usize,
    pub sink_node_count: usize,
    pub isolated_node_count: usize,
    pub self_relation_count: usize,
    pub context_count: usize,
    pub semantic_count: usize,
}

#[derive(Clone, Copy)]
pub struct ConsistencyReport {
    pub valid: bool,
    pub checked_relations: usize,
    pub invalid_relations: usize,
    pub duplicate_relations: usize,
    pub disconnected_relation_count: usize,
}

fn pattern_matches(id: u64, pattern: &GraphPattern) -> bool {
    let mut matches = false;
    let _ = relation::with_relation(id, |relation| {
        matches = pattern.source.map_or(true, |v| v == relation.source)
            && pattern.relation.map_or(true, |v| v == relation.relation)
            && pattern.target.map_or(true, |v| v == relation.target)
            && pattern.context.map_or(true, |v| v == relation.context);
    });
    matches
}

fn contains_node(snapshot: &GraphSnapshot, node: ObjectId) -> bool {
    let mut i = 0usize;
    while i < snapshot.node_count {
        if snapshot.nodes[i] == node {
            return true;
        }
        i += 1;
    }
    false
}

fn add_node(snapshot: &mut GraphSnapshot, node: ObjectId) -> bool {
    if node == 0 || contains_node(snapshot, node) {
        return true;
    }
    if snapshot.node_count >= MAX_GRAPH_NODES {
        return false;
    }
    snapshot.nodes[snapshot.node_count] = node;
    snapshot.node_count += 1;
    true
}

fn add_relation(snapshot: &mut GraphSnapshot, id: u64) -> bool {
    if id == 0 {
        return false;
    }
    let mut i = 0usize;
    while i < snapshot.relation_count {
        if snapshot.relation_ids[i] == id {
            return true;
        }
        i += 1;
    }
    if snapshot.relation_count >= MAX_GRAPH_RELATIONS {
        return false;
    }
    snapshot.relation_ids[snapshot.relation_count] = id;
    snapshot.relation_count += 1;
    true
}

/// Builds a deterministic snapshot of the complete active relation graph.
pub fn snapshot() -> Option<GraphSnapshot> {
    if relation::count() == 0 {
        return None;
    }
    let mut result = GraphSnapshot::empty();
    let mut ok = true;
    relation::for_each(|id, relation| {
        if !ok {
            return;
        }
        if !add_relation(&mut result, id)
            || !add_node(&mut result, relation.source)
            || !add_node(&mut result, relation.target)
        {
            ok = false;
        }
    });
    if ok && result.is_valid() {
        Some(result)
    } else {
        None
    }
}

/// Extracts the bounded one-hop neighborhood around a node.
pub fn neighborhood(node: ObjectId, direction: GraphDirection) -> Option<GraphSnapshot> {
    if node == 0 || relation::count() == 0 {
        return None;
    }
    let mut result = GraphSnapshot::empty();
    if !add_node(&mut result, node) {
        return None;
    }
    let mut ok = true;
    relation::for_each(|id, relation| {
        if !ok {
            return;
        }
        let include = match direction {
            GraphDirection::Outgoing => relation.source == node,
            GraphDirection::Incoming => relation.target == node,
            GraphDirection::Both => relation.source == node || relation.target == node,
        };
        if include
            && (!add_relation(&mut result, id)
                || !add_node(&mut result, relation.source)
                || !add_node(&mut result, relation.target))
        {
            ok = false;
        }
    });
    if ok && result.relation_count != 0 && result.is_valid() {
        Some(result)
    } else {
        None
    }
}

/// Returns all relation IDs matching a graph pattern in registry order.
pub fn match_pattern(pattern: GraphPattern, output: &mut [u64]) -> usize {
    let mut count = 0usize;
    relation::for_each(|id, _| {
        if count < output.len() && pattern_matches(id, &pattern) {
            output[count] = id;
            count += 1;
        }
    });
    count
}

/// Computes deterministic graph statistics from the active relation registry.
pub fn stats() -> GraphStats {
    let relation_count = relation::count();
    let mut nodes = [0u64; MAX_GRAPH_NODES];
    let mut outgoing = [false; MAX_GRAPH_NODES];
    let mut incoming = [false; MAX_GRAPH_NODES];
    let mut contexts = [0u64; MAX_GRAPH_RELATIONS];
    let mut semantics = [0u32; MAX_GRAPH_RELATIONS];
    let mut node_count = 0usize;
    let mut context_count = 0usize;
    let mut semantic_count = 0usize;
    let mut source_node_count = 0usize;
    let mut sink_node_count = 0usize;
    let mut self_relation_count = 0usize;

    relation::for_each(|_, relation| {
        if relation.source == relation.target {
            self_relation_count += 1;
        }
        let mut source_index = usize::MAX;
        let mut target_index = usize::MAX;
        let mut i = 0usize;
        while i < node_count {
            if nodes[i] == relation.source {
                source_index = i;
            }
            if nodes[i] == relation.target {
                target_index = i;
            }
            i += 1;
        }
        if source_index == usize::MAX && node_count < MAX_GRAPH_NODES {
            source_index = node_count;
            nodes[node_count] = relation.source;
            node_count += 1;
        }
        if target_index == usize::MAX && node_count < MAX_GRAPH_NODES {
            target_index = node_count;
            nodes[node_count] = relation.target;
            node_count += 1;
        }
        if source_index != usize::MAX {
            outgoing[source_index] = true;
        }
        if target_index != usize::MAX {
            incoming[target_index] = true;
        }

        let mut found_context = false;
        i = 0;
        while i < context_count {
            if contexts[i] == relation.context {
                found_context = true;
                break;
            }
            i += 1;
        }
        if !found_context && context_count < MAX_GRAPH_RELATIONS {
            contexts[context_count] = relation.context;
            context_count += 1;
        }

        let mut found_semantic = false;
        i = 0;
        while i < semantic_count {
            if semantics[i] == relation.relation {
                found_semantic = true;
                break;
            }
            i += 1;
        }
        if !found_semantic && semantic_count < MAX_GRAPH_RELATIONS {
            semantics[semantic_count] = relation.relation;
            semantic_count += 1;
        }
    });

    let mut isolated_node_count = 0usize;
    let mut i = 0usize;
    while i < node_count {
        if !outgoing[i] && !incoming[i] {
            isolated_node_count += 1;
        }
        if outgoing[i] {
            source_node_count += 1;
        }
        if incoming[i] {
            sink_node_count += 1;
        }
        i += 1;
    }

    GraphStats {
        relation_count,
        node_count,
        source_node_count,
        sink_node_count,
        isolated_node_count,
        self_relation_count,
        context_count,
        semantic_count,
    }
}

/// Validates relation structure, duplicate records, and endpoint consistency.
pub fn validate() -> ConsistencyReport {
    let mut report = ConsistencyReport {
        valid: true,
        checked_relations: 0,
        invalid_relations: 0,
        duplicate_relations: 0,
        disconnected_relation_count: 0,
    };
    let mut seen = [0u64; MAX_GRAPH_RELATIONS];
    let mut seen_count = 0usize;

    relation::for_each(|id, relation| {
        report.checked_relations += 1;
        if !relation::is_valid(relation) || !relation::contains(id) {
            report.invalid_relations += 1;
            report.valid = false;
        }
        let mut i = 0usize;
        while i < seen_count {
            if seen[i] == id {
                report.duplicate_relations += 1;
                report.valid = false;
                break;
            }
            i += 1;
        }
        if seen_count < MAX_GRAPH_RELATIONS {
            seen[seen_count] = id;
            seen_count += 1;
        }
        if relation::find_by_source(relation.source).is_none()
            || relation::find_by_target(relation.target).is_none()
            || relation::find_by_semantic(relation.relation).is_none()
        {
            report.disconnected_relation_count += 1;
            report.valid = false;
        }
    });
    report
}

/// Returns the number of nodes in the connected component containing `start`.
pub fn component_size(start: ObjectId, direction: GraphDirection) -> Option<usize> {
    if start == 0 || relation::count() == 0 {
        return None;
    }
    let mut visited = [0u64; MAX_GRAPH_NODES];
    let mut queue = [0u64; MAX_GRAPH_NODES];
    let mut visited_count = 1usize;
    let mut head = 0usize;
    let mut tail = 1usize;
    queue[0] = start;
    visited[0] = start;

    while head < tail {
        let current = queue[head];
        head += 1;
        relation::for_each(|_, relation| {
            let next = match direction {
                GraphDirection::Outgoing if relation.source == current => Some(relation.target),
                GraphDirection::Incoming if relation.target == current => Some(relation.source),
                GraphDirection::Both if relation.source == current => Some(relation.target),
                GraphDirection::Both if relation.target == current => Some(relation.source),
                _ => None,
            };
            if let Some(next) = next {
                let mut seen = false;
                let mut i = 0usize;
                while i < visited_count {
                    if visited[i] == next {
                        seen = true;
                        break;
                    }
                    i += 1;
                }
                if !seen && visited_count < MAX_GRAPH_NODES {
                    visited[visited_count] = next;
                    visited_count += 1;
                    queue[tail] = next;
                    tail += 1;
                }
            }
        });
    }
    Some(visited_count)
}

fn cleanup_relations_registry() -> bool {
    let mut ids = [0u64; MAX_GRAPH_RELATIONS];
    let mut count = 0usize;

    relation::for_each(|id, _| {
        if count < ids.len() {
            ids[count] = id;
            count += 1;
        }
    });

    let mut index = 0usize;
    while index < count {
        if !relation::remove(ids[index]) {
            return false;
        }
        index += 1;
    }

    relation::count() == 0
}

pub fn self_test() -> bool {
    // Keep graph tests isolated from relation-layer self-test breadth.
    if !cleanup_relations_registry() {
        return false;
    }

    if relation::count() != 0 {
        return false;
    }

    let a = relation::Relation {
        source: 10,
        relation: super::semantic::CONNECTED_TO,
        target: 20,
        context: 1,
    };
    let b = relation::Relation {
        source: 20,
        relation: super::semantic::CONNECTED_TO,
        target: 30,
        context: 1,
    };
    let c = relation::Relation {
        source: 30,
        relation: super::semantic::CAUSES,
        target: 40,
        context: 2,
    };
    let ia = match relation::add(a) {
        Some(v) => v,
        None => return false,
    };
    let ib = match relation::add(b) {
        Some(v) => v,
        None => {
            relation::remove(ia);
            return false;
        }
    };
    let ic = match relation::add(c) {
        Some(v) => v,
        None => {
            relation::remove(ia);
            relation::remove(ib);
            return false;
        }
    };

    let full = match snapshot() {
        Some(v) => v,
        None => {
            cleanup_relations_registry();
            return false;
        }
    };
    if full.relation_count != 3 || full.node_count != 4 || !full.is_valid() {
        cleanup_relations_registry();
        return false;
    }

    let near = match neighborhood(20, GraphDirection::Both) {
        Some(v) => v,
        None => {
            cleanup_relations_registry();
            return false;
        }
    };
    if near.relation_count != 2 || near.node_count != 3 {
        cleanup_relations_registry();
        return false;
    }

    let mut matches = [0u64; MAX_GRAPH_RELATIONS];
    let matched = match_pattern(
        GraphPattern {
            source: None,
            relation: Some(super::semantic::CONNECTED_TO),
            target: None,
            context: Some(1),
        },
        &mut matches,
    );
    if matched != 2 || matches[0] != ia || matches[1] != ib {
        cleanup_relations_registry();
        return false;
    }

    let s = stats();
    if s.relation_count != 3
        || s.node_count != 4
        || s.self_relation_count != 0
        || s.context_count != 2
        || s.semantic_count != 2
    {
        relation::remove(ia);
        relation::remove(ib);
        relation::remove(ic);
        return false;
    }

    let report = validate();
    if !report.valid
        || report.checked_relations != 3
        || report.invalid_relations != 0
        || report.duplicate_relations != 0
    {
        cleanup_relations_registry();
        return false;
    }

    if component_size(10, GraphDirection::Outgoing) != Some(4)
        || component_size(40, GraphDirection::Outgoing) != Some(1)
        || component_size(30, GraphDirection::Incoming) != Some(3)
    {
        cleanup_relations_registry();
        return false;
    }

    if !relation::remove(ia) || !relation::remove(ib) || !relation::remove(ic) {
        cleanup_relations_registry();
        return false;
    }

    relation::count() == 0
}
