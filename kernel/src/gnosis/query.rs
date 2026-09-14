#![allow(clippy::missing_safety_doc)]

//! GNOSIS Graph Query Engine — bounded deterministic queries over the knowledge graph.
//!
//! REL-01.9 adds multi-hop query execution, pattern chains, bounded path results,
//! deterministic scoring, query validation, and a self-test without changing the
//! canonical relation or graph storage models.

use super::graph::GraphDirection;
use super::relation;
use super::semantic::{ContextId, ObjectId, SemanticId};

pub const MAX_QUERY_DEPTH: usize = 16;
pub const MAX_QUERY_PATHS: usize = 32;
pub const MAX_QUERY_RESULTS: usize = MAX_QUERY_PATHS;

#[derive(Clone, Copy)]
pub struct QueryStep {
    pub relation: Option<SemanticId>,
    pub context: Option<ContextId>,
    pub target: Option<ObjectId>,
}

impl QueryStep {
    pub const fn any() -> Self {
        Self {
            relation: None,
            context: None,
            target: None,
        }
    }
}

#[derive(Clone, Copy)]
pub struct GraphQuery {
    pub source: ObjectId,
    pub target: Option<ObjectId>,
    pub direction: GraphDirection,
    pub max_depth: usize,
    pub relation: Option<SemanticId>,
    pub context: Option<ContextId>,
}

#[derive(Clone, Copy)]
pub struct PatternQuery {
    pub source: ObjectId,
    pub steps: [QueryStep; MAX_QUERY_DEPTH],
    pub step_count: usize,
}

impl PatternQuery {
    pub const fn empty(source: ObjectId) -> Self {
        Self {
            source,
            steps: [QueryStep::any(); MAX_QUERY_DEPTH],
            step_count: 0,
        }
    }

    pub fn push(&mut self, step: QueryStep) -> bool {
        if self.step_count >= MAX_QUERY_DEPTH {
            return false;
        }
        self.steps[self.step_count] = step;
        self.step_count += 1;
        true
    }

    pub fn is_valid(&self) -> bool {
        self.source != 0 && self.step_count > 0 && self.step_count <= MAX_QUERY_DEPTH
    }
}

#[derive(Clone, Copy)]
pub struct QueryPath {
    pub nodes: [ObjectId; MAX_QUERY_DEPTH + 1],
    pub relation_ids: [u64; MAX_QUERY_DEPTH],
    pub node_count: usize,
    pub relation_count: usize,
    pub score: u16,
}

impl QueryPath {
    const fn empty(source: ObjectId) -> Self {
        let mut path = Self {
            nodes: [0; MAX_QUERY_DEPTH + 1],
            relation_ids: [0; MAX_QUERY_DEPTH],
            node_count: 0,
            relation_count: 0,
            score: 0,
        };
        path.nodes[0] = source;
        path.node_count = 1;
        path
    }

    pub fn is_valid(&self) -> bool {
        if self.node_count == 0
            || self.node_count > MAX_QUERY_DEPTH + 1
            || self.relation_count + 1 != self.node_count
            || self.relation_count > MAX_QUERY_DEPTH
        {
            return false;
        }
        let mut i = 0usize;
        while i < self.node_count {
            if self.nodes[i] == 0 {
                return false;
            }
            i += 1;
        }
        i = 0;
        while i < self.relation_count {
            if self.relation_ids[i] == 0 {
                return false;
            }
            i += 1;
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
pub struct QueryReport {
    pub valid: bool,
    pub paths_found: usize,
    pub relations_examined: usize,
    pub rejected_paths: usize,
}

fn step_matches(relation: &relation::Relation, step: QueryStep) -> bool {
    step.relation
        .map_or(true, |value| value == relation.relation)
        && step.context.map_or(true, |value| value == relation.context)
        && step.target.map_or(true, |value| value == relation.target)
}

fn edge_matches(
    relation: &relation::Relation,
    current: ObjectId,
    direction: GraphDirection,
    semantic: Option<SemanticId>,
    context: Option<ContextId>,
    target: Option<ObjectId>,
) -> Option<ObjectId> {
    if semantic.map_or(false, |value| value != relation.relation)
        || context.map_or(false, |value| value != relation.context)
    {
        return None;
    }

    match direction {
        GraphDirection::Outgoing if relation.source == current => {
            if target.map_or(true, |value| value == relation.target) {
                Some(relation.target)
            } else {
                None
            }
        }
        GraphDirection::Incoming if relation.target == current => {
            if target.map_or(true, |value| value == relation.source) {
                Some(relation.source)
            } else {
                None
            }
        }
        GraphDirection::Both if relation.source == current => {
            if target.map_or(true, |value| value == relation.target) {
                Some(relation.target)
            } else {
                None
            }
        }
        GraphDirection::Both if relation.target == current => {
            if target.map_or(true, |value| value == relation.source) {
                Some(relation.source)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn path_contains(path: &QueryPath, node: ObjectId) -> bool {
    let mut i = 0usize;
    while i < path.node_count {
        if path.nodes[i] == node {
            return true;
        }
        i += 1;
    }
    false
}

fn relation_score(id: u64) -> u16 {
    if !relation::verify_integrity(id) {
        return 0;
    }
    match relation::trust(id) {
        Some(relation::RelationTrust::Verified) => 100,
        Some(relation::RelationTrust::Unverified) => 50,
        Some(relation::RelationTrust::Conflict) => 10,
        Some(relation::RelationTrust::Invalid) => 0,
        None => 0,
    }
}

fn extend_path(path: &QueryPath, relation_id: u64, target: ObjectId) -> Option<QueryPath> {
    if target == 0 || path.relation_count >= MAX_QUERY_DEPTH || path_contains(path, target) {
        return None;
    }
    let mut next = *path;
    next.relation_ids[next.relation_count] = relation_id;
    next.nodes[next.node_count] = target;
    next.relation_count += 1;
    next.node_count += 1;
    next.score = next.score.saturating_add(relation_score(relation_id));
    Some(next)
}

/// Validates a general graph query before execution.
pub fn validate_query(query: GraphQuery) -> bool {
    query.source != 0
        && query.max_depth > 0
        && query.max_depth <= MAX_QUERY_DEPTH
        && query.target != Some(0)
}

/// Executes a bounded multi-hop query and writes deterministic paths to `output`.
/// Results are ordered by relation-registry order and expansion order.
pub fn find_paths(query: GraphQuery, output: &mut [QueryPath]) -> QueryReport {
    let mut report = QueryReport {
        valid: validate_query(query),
        paths_found: 0,
        relations_examined: 0,
        rejected_paths: 0,
    };
    if !report.valid || output.is_empty() {
        return report;
    }

    let mut frontier = [QueryPath::empty(query.source); MAX_QUERY_PATHS];
    let mut frontier_count = 1usize;
    let mut depth = 0usize;

    if query.target == Some(query.source) {
        output[0] = frontier[0];
        report.paths_found = 1;
        return report;
    }

    while depth < query.max_depth && frontier_count > 0 {
        let mut next_frontier = [QueryPath::empty(0); MAX_QUERY_PATHS];
        let mut next_count = 0usize;
        let mut i = 0usize;

        while i < frontier_count {
            let current_path = frontier[i];
            let current = current_path.nodes[current_path.node_count - 1];
            relation::for_each(|id, relation| {
                report.relations_examined += 1;
                if next_count >= MAX_QUERY_PATHS {
                    return;
                }
                let target = match edge_matches(
                    relation,
                    current,
                    query.direction,
                    query.relation,
                    query.context,
                    None,
                ) {
                    Some(value) => value,
                    None => return,
                };
                let candidate = match extend_path(&current_path, id, target) {
                    Some(value) => value,
                    None => {
                        report.rejected_paths += 1;
                        return;
                    }
                };
                if query.target == Some(target) {
                    if report.paths_found < output.len() {
                        output[report.paths_found] = candidate;
                        report.paths_found += 1;
                    }
                } else if next_count < MAX_QUERY_PATHS {
                    next_frontier[next_count] = candidate;
                    next_count += 1;
                }
            });
            i += 1;
        }

        frontier = next_frontier;
        frontier_count = next_count;
        depth += 1;
        if report.paths_found >= output.len() {
            break;
        }
    }

    report
}

/// Executes an exact ordered pattern chain. Each step constrains relation, context,
/// and optionally the target node.
pub fn match_pattern(query: PatternQuery, output: &mut [QueryPath]) -> QueryReport {
    let mut report = QueryReport {
        valid: query.is_valid(),
        paths_found: 0,
        relations_examined: 0,
        rejected_paths: 0,
    };
    if !report.valid || output.is_empty() {
        return report;
    }

    let mut frontier = [QueryPath::empty(query.source); MAX_QUERY_PATHS];
    let mut frontier_count = 1usize;
    let mut step_index = 0usize;

    while step_index < query.step_count && frontier_count > 0 {
        let step = query.steps[step_index];
        let mut next_frontier = [QueryPath::empty(0); MAX_QUERY_PATHS];
        let mut next_count = 0usize;
        let mut i = 0usize;

        while i < frontier_count {
            let current_path = frontier[i];
            let current = current_path.nodes[current_path.node_count - 1];
            relation::for_each(|id, relation| {
                report.relations_examined += 1;
                if next_count >= MAX_QUERY_PATHS {
                    return;
                }
                if relation.source != current || !step_matches(relation, step) {
                    return;
                }
                let candidate = match extend_path(&current_path, id, relation.target) {
                    Some(value) => value,
                    None => {
                        report.rejected_paths += 1;
                        return;
                    }
                };
                next_frontier[next_count] = candidate;
                next_count += 1;
            });
            i += 1;
        }

        frontier = next_frontier;
        frontier_count = next_count;
        step_index += 1;
    }

    let mut i = 0usize;
    while i < frontier_count && report.paths_found < output.len() {
        output[report.paths_found] = frontier[i];
        report.paths_found += 1;
        i += 1;
    }
    report
}

/// Returns true when the target is reachable within the bounded query depth.
pub fn reachable(query: GraphQuery) -> bool {
    let mut output = [QueryPath::empty(0); 1];
    find_paths(query, &mut output).paths_found != 0
}

/// Returns the highest deterministic score among paths found by `query`.
pub fn best_score(query: GraphQuery) -> Option<u16> {
    let mut output = [QueryPath::empty(0); MAX_QUERY_PATHS];
    let report = find_paths(query, &mut output);
    if report.paths_found == 0 {
        return None;
    }
    let mut best = 0u16;
    let mut i = 0usize;
    while i < report.paths_found {
        if output[i].score > best {
            best = output[i].score;
        }
        i += 1;
    }
    Some(best)
}

/// Validates query-result structure and score consistency.
pub fn validate_path(path: &QueryPath) -> bool {
    if !path.is_valid() {
        return false;
    }
    let mut expected = 0u16;
    let mut i = 0usize;
    while i < path.relation_count {
        expected = expected.saturating_add(relation_score(path.relation_ids[i]));
        i += 1;
    }
    path.score == expected
}

/// Runs deterministic multi-hop, pattern, scoring, and result-integrity tests.
pub fn self_test() -> bool {
    // Query self-test owns query behavior; the relation engine has its own
    // isolated self-test and must not be coupled into this layer.
    if relation::count() != 0 {
        return false;
    }

    let r1 = match relation::add(relation::Relation {
        source: 10,
        relation: super::semantic::CONNECTED_TO,
        target: 20,
        context: 1,
    }) {
        Some(id) => id,
        None => return false,
    };
    let r2 = match relation::add(relation::Relation {
        source: 20,
        relation: super::semantic::CONNECTED_TO,
        target: 30,
        context: 1,
    }) {
        Some(id) => id,
        None => {
            relation::remove(r1);
            return false;
        }
    };
    let r3 = match relation::add(relation::Relation {
        source: 30,
        relation: super::semantic::CAUSES,
        target: 40,
        context: 2,
    }) {
        Some(id) => id,
        None => {
            relation::remove(r1);
            relation::remove(r2);
            return false;
        }
    };

    let query = GraphQuery {
        source: 10,
        target: Some(40),
        direction: GraphDirection::Outgoing,
        max_depth: 4,
        relation: None,
        context: None,
    };
    let mut paths = [QueryPath::empty(0); MAX_QUERY_PATHS];
    let report = find_paths(query, &mut paths);
    if !report.valid || report.paths_found != 1 || !validate_path(&paths[0]) {
        return false;
    }
    if paths[0].node_count != 4 || paths[0].relation_count != 3 {
        return false;
    }
    if paths[0].nodes[0] != 10 || paths[0].nodes[3] != 40 {
        return false;
    }
    if paths[0].relation_ids[0] != r1
        || paths[0].relation_ids[1] != r2
        || paths[0].relation_ids[2] != r3
    {
        return false;
    }
    if best_score(query).is_none() || !reachable(query) {
        return false;
    }

    let mut pattern = PatternQuery::empty(10);
    if !pattern.push(QueryStep {
        relation: Some(super::semantic::CONNECTED_TO),
        context: Some(1),
        target: Some(20),
    }) {
        return false;
    }
    if !pattern.push(QueryStep {
        relation: Some(super::semantic::CONNECTED_TO),
        context: Some(1),
        target: Some(30),
    }) {
        return false;
    }
    if !pattern.push(QueryStep {
        relation: Some(super::semantic::CAUSES),
        context: Some(2),
        target: Some(40),
    }) {
        return false;
    }

    let mut pattern_paths = [QueryPath::empty(0); MAX_QUERY_PATHS];
    let pattern_report = match_pattern(pattern, &mut pattern_paths);
    if !pattern_report.valid || pattern_report.paths_found != 1 || !validate_path(&pattern_paths[0])
    {
        return false;
    }

    let invalid = GraphQuery {
        source: 0,
        target: Some(40),
        direction: GraphDirection::Outgoing,
        max_depth: 4,
        relation: None,
        context: None,
    };
    if validate_query(invalid) || reachable(invalid) {
        return false;
    }

    relation::remove(r1);
    relation::remove(r2);
    relation::remove(r3);
    relation::count() == 0
}
