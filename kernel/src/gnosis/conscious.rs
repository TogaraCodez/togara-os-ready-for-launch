//! GNOSIS Conscious Workspace Engine — REL-10
//!
//! The Conscious Workspace is a bounded derived integration layer between
//! Attention/Subconscious signals and later deliberative layers.
//!
//! Contract:
//! - canonical GNOSIS state is never mutated;
//! - Subconscious and Attention remain authoritative for their own state;
//! - workspace state is derived, ephemeral, fixed-capacity, and deterministic;
//! - no heap allocation is used;
//! - admission and ranking are deterministic;
//! - uncertainty is preserved rather than converted into a decision;
//! - the workspace does not execute actions or select policy.
//!
//! Flow:
//!
//!     GNOSIS / Delta
//!          |
//!       Attention ----+
//!          |          |
//!       Subconscious  |
//!          |          |
//!          +----> Conscious Workspace
//!                         |
//!                  bounded active scene
//!                         |
//!                    later deliberation

use super::attention::AttentionRecord;
use super::relation::{ProvenanceSource, RelationProvenance};
use super::semantic::{ContextId, ObjectId, SemanticId};
use super::subconscious::{self, CandidateKind, CandidateRecord};
use spin::Mutex;

pub const SCALE: u32 = 10_000;
pub const MAX_WORKSPACE: usize = 16;
pub const MAX_SEEN: usize = 64;
pub const MAX_WORK: usize = 2048;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceSource {
    Attention,
    Subconscious,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkspaceItem {
    pub object_id: ObjectId,
    pub semantic_id: SemanticId,
    pub previous_state: SemanticId,
    pub current_state: SemanticId,
    pub predicted_state: SemanticId,
    pub context: ContextId,
    pub salience_fixed: u32,
    pub uncertainty_fixed: u32,
    pub support: u32,
    pub timestamp: u64,
    pub provenance: RelationProvenance,
    pub source: WorkspaceSource,
}

impl WorkspaceItem {
    pub const fn empty() -> Self {
        Self {
            object_id: 0,
            semantic_id: 0,
            previous_state: 0,
            current_state: 0,
            predicted_state: 0,
            context: 0,
            salience_fixed: 0,
            uncertainty_fixed: SCALE,
            support: 0,
            timestamp: 0,
            provenance: RelationProvenance {
                origin: 0,
                source: ProvenanceSource::Local,
                revision: 0,
            },
            source: WorkspaceSource::Attention,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SeenKey {
    object_id: ObjectId,
    semantic_id: SemanticId,
    previous_state: SemanticId,
    current_state: SemanticId,
    predicted_state: SemanticId,
    context: ContextId,
    timestamp: u64,
    origin: u64,
    revision: u64,
    source: WorkspaceSource,
}

impl SeenKey {
    #[inline]
    const fn from_item(item: &WorkspaceItem) -> Self {
        Self {
            object_id: item.object_id,
            semantic_id: item.semantic_id,
            previous_state: item.previous_state,
            current_state: item.current_state,
            predicted_state: item.predicted_state,
            context: item.context,
            timestamp: item.timestamp,
            origin: item.provenance.origin,
            revision: item.provenance.revision,
            source: item.source,
        }
    }
}

#[derive(Clone, Copy)]
struct WorkspaceState {
    items: [WorkspaceItem; MAX_WORKSPACE],
    item_used: usize,
    seen: [SeenKey; MAX_SEEN],
    seen_used: usize,
    cycle: u64,
}

impl WorkspaceState {
    const fn empty() -> Self {
        Self {
            items: [WorkspaceItem::empty(); MAX_WORKSPACE],
            item_used: 0,
            seen: [SeenKey {
                object_id: 0,
                semantic_id: 0,
                previous_state: 0,
                current_state: 0,
                predicted_state: 0,
                context: 0,
                timestamp: 0,
                origin: 0,
                revision: 0,
                source: WorkspaceSource::Attention,
            }; MAX_SEEN],
            seen_used: 0,
            cycle: 0,
        }
    }
}

static WORKSPACE: Mutex<WorkspaceState> = Mutex::new(WorkspaceState::empty());

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConsciousReport {
    pub admitted: usize,
    pub rejected: usize,
    pub duplicate: usize,
    pub workspace_count: usize,
    pub work_used: usize,
    pub work_budget: usize,
    pub stopped_by_budget: bool,
    pub overflow: bool,
    pub focused: WorkspaceItem,
}

impl ConsciousReport {
    pub const fn empty() -> Self {
        Self {
            admitted: 0,
            rejected: 0,
            duplicate: 0,
            workspace_count: 0,
            work_used: 0,
            work_budget: MAX_WORK,
            stopped_by_budget: false,
            overflow: false,
            focused: WorkspaceItem::empty(),
        }
    }
}

#[inline]
fn charge(report: &mut ConsciousReport, amount: usize) -> bool {
    let next = report.work_used.saturating_add(amount);
    if next > report.work_budget {
        report.work_used = report.work_budget;
        report.stopped_by_budget = true;
        report.overflow = true;
        return false;
    }
    report.work_used = next;
    true
}

#[inline]
fn source_rank(source: WorkspaceSource) -> u8 {
    match source {
        WorkspaceSource::Attention => 0,
        WorkspaceSource::Subconscious => 1,
    }
}

/// Total ordering used for deterministic workspace competition.
/// Higher salience wins. Equal salience uses lower uncertainty, higher
/// support, newer timestamp, then stable identity fields.
#[inline]
fn outranks(a: &WorkspaceItem, b: &WorkspaceItem) -> bool {
    if a.salience_fixed != b.salience_fixed {
        return a.salience_fixed > b.salience_fixed;
    }
    if a.uncertainty_fixed != b.uncertainty_fixed {
        return a.uncertainty_fixed < b.uncertainty_fixed;
    }
    if a.support != b.support {
        return a.support > b.support;
    }
    if a.timestamp != b.timestamp {
        return a.timestamp > b.timestamp;
    }
    if a.object_id != b.object_id {
        return a.object_id < b.object_id;
    }
    if a.semantic_id != b.semantic_id {
        return a.semantic_id < b.semantic_id;
    }
    if a.context != b.context {
        return a.context < b.context;
    }
    if a.previous_state != b.previous_state {
        return a.previous_state < b.previous_state;
    }
    if a.current_state != b.current_state {
        return a.current_state < b.current_state;
    }
    if a.predicted_state != b.predicted_state {
        return a.predicted_state < b.predicted_state;
    }
    source_rank(a.source) < source_rank(b.source)
}

#[inline]
fn contains_seen(state: &WorkspaceState, key: &SeenKey) -> bool {
    let mut index = 0usize;
    while index < state.seen_used {
        if state.seen[index] == *key {
            return true;
        }
        index += 1;
    }
    false
}

#[inline]
fn remember_seen(state: &mut WorkspaceState, key: SeenKey) {
    if state.seen_used < MAX_SEEN {
        state.seen[state.seen_used] = key;
        state.seen_used += 1;
        return;
    }

    // Deterministic ring-like replacement using cycle modulo capacity.
    let index = (state.cycle as usize) % MAX_SEEN;
    state.seen[index] = key;
}

#[inline]
fn candidate_salience(candidate: &CandidateRecord) -> u32 {
    let kind_weight: u32 = match candidate.kind {
        CandidateKind::Anomaly => 10_000,
        CandidateKind::Prediction => 8_000,
        CandidateKind::ResourceTransition => 6_000,
        CandidateKind::RepeatedTransition => 4_000,
    };

    let confidence = candidate.score_fixed.min(SCALE);
    let base = kind_weight.saturating_add(confidence);
    base.min(20_000)
}

#[inline]
fn from_candidate(candidate: &CandidateRecord) -> WorkspaceItem {
    WorkspaceItem {
        object_id: candidate.origin_object_id,
        semantic_id: candidate.semantic_id,
        previous_state: candidate.previous_state,
        current_state: candidate.observed_state,
        predicted_state: candidate.predicted_state,
        context: candidate.context,
        salience_fixed: candidate_salience(candidate),
        uncertainty_fixed: candidate.uncertainty_fixed.min(SCALE),
        support: candidate.support,
        timestamp: candidate.timestamp,
        provenance: candidate.provenance,
        source: WorkspaceSource::Subconscious,
    }
}

#[inline]
fn from_attention(record: &AttentionRecord) -> WorkspaceItem {
    WorkspaceItem {
        object_id: record.object_id,
        semantic_id: record.semantic_id,
        previous_state: record.previous_state,
        current_state: record.new_state,
        predicted_state: record.new_state,
        context: record.context,
        salience_fixed: u32::from(record.score).saturating_mul(10),
        uncertainty_fixed: SCALE
            .saturating_sub(u32::from(record.score).saturating_mul(10).min(SCALE)),
        support: 1,
        timestamp: record.timestamp,
        provenance: record.provenance,
        source: WorkspaceSource::Attention,
    }
}

#[inline]
fn admit(state: &mut WorkspaceState, item: WorkspaceItem) -> bool {
    if state.item_used < MAX_WORKSPACE {
        state.items[state.item_used] = item;
        state.item_used += 1;
        return true;
    }

    let mut worst = 0usize;
    let mut index = 1usize;
    while index < MAX_WORKSPACE {
        if outranks(&state.items[worst], &state.items[index]) {
            worst = index;
        }
        index += 1;
    }

    if outranks(&item, &state.items[worst]) {
        state.items[worst] = item;
        true
    } else {
        false
    }
}

#[inline]
fn focus(state: &WorkspaceState) -> WorkspaceItem {
    if state.item_used == 0 {
        return WorkspaceItem::empty();
    }

    let mut best = 0usize;
    let mut index = 1usize;
    while index < state.item_used {
        if outranks(&state.items[index], &state.items[best]) {
            best = index;
        }
        index += 1;
    }
    state.items[best]
}

/// Clears only the derived conscious workspace.
#[inline]
pub fn clear() {
    *WORKSPACE.lock() = WorkspaceState::empty();
}

#[inline]
pub fn count() -> usize {
    WORKSPACE.lock().item_used
}

/// Visits the active workspace in deterministic ranking order without
/// consuming or mutating the workspace.
pub fn for_each<F: FnMut(&WorkspaceItem)>(mut callback: F) {
    let state = WORKSPACE.lock();
    let mut used = [false; MAX_WORKSPACE];
    let mut rank = 0usize;

    while rank < state.item_used {
        let mut selected: Option<usize> = None;
        let mut index = 0usize;
        while index < state.item_used {
            if !used[index]
                && (selected.is_none()
                    || outranks(&state.items[index], &state.items[selected.unwrap()]))
            {
                selected = Some(index);
            }
            index += 1;
        }

        if let Some(index) = selected {
            used[index] = true;
            callback(&state.items[index]);
        }
        rank += 1;
    }
}

/// Integrates current derived Attention and Subconscious signals into the
/// bounded conscious scene. Canonical GNOSIS state is never modified.
#[inline]
pub fn process() -> ConsciousReport {
    let mut report = ConsciousReport::empty();
    let mut state = WORKSPACE.lock();
    state.cycle = state.cycle.saturating_add(1);

    let mut attention_items = [WorkspaceItem::empty(); super::attention::MAX_ATTENTION];
    let mut attention_count = 0usize;

    super::attention::for_each(|record| {
        if attention_count < attention_items.len() {
            attention_items[attention_count] = from_attention(record);
            attention_count += 1;
        }
    });

    let mut index = 0usize;
    while index < attention_count {
        if !charge(&mut report, 2) {
            report.workspace_count = state.item_used;
            report.focused = focus(&state);
            return report;
        }

        let item = attention_items[index];
        let key = SeenKey::from_item(&item);
        if contains_seen(&state, &key) {
            report.duplicate = report.duplicate.saturating_add(1);
        } else {
            if admit(&mut state, item) {
                report.admitted = report.admitted.saturating_add(1);
            } else {
                report.rejected = report.rejected.saturating_add(1);
            }
            remember_seen(&mut state, key);
        }
        index += 1;
    }

    let mut candidate_snapshot = [CandidateRecord::empty(); subconscious::MAX_CANDIDATES];

    let candidate_count = subconscious::latest_candidates(&mut candidate_snapshot);

    let mut candidate_index = 0usize;

    while candidate_index < candidate_count {
        if !charge(&mut report, 2) {
            report.workspace_count = state.item_used;
            report.focused = focus(&state);
            return report;
        }

        let candidate = candidate_snapshot[candidate_index];
        let item = from_candidate(&candidate);
        let key = SeenKey::from_item(&item);

        if contains_seen(&state, &key) {
            report.duplicate = report.duplicate.saturating_add(1);
        } else if admit(&mut state, item) {
            report.admitted = report.admitted.saturating_add(1);
            remember_seen(&mut state, key);
        } else {
            report.rejected = report.rejected.saturating_add(1);
            remember_seen(&mut state, key);
        }

        candidate_index += 1;
    }

    if candidate_count >= subconscious::MAX_CANDIDATES && !charge(&mut report, 1) {
        report.workspace_count = state.item_used;
        report.focused = focus(&state);
        return report;
    }

    report.workspace_count = state.item_used;
    report.focused = focus(&state);
    report
}

/// Deterministic bounded integration self-test.
#[inline]
pub fn self_test() -> bool {
    clear();

    let attention = AttentionRecord {
        object_id: 10,
        semantic_id: super::semantic::ACTIVE,
        previous_state: super::semantic::INACTIVE,
        new_state: super::semantic::ACTIVE,
        context: 1,
        timestamp: 10,
        provenance: RelationProvenance {
            origin: 10,
            source: ProvenanceSource::Local,
            revision: 1,
        },
        score: 1000,
    };

    let candidate = CandidateRecord {
        kind: CandidateKind::Anomaly,
        semantic_id: super::semantic::ACTIVE,
        previous_state: super::semantic::ACTIVE,
        observed_state: super::semantic::FAILED,
        predicted_state: super::semantic::ACTIVE,
        context: 1,
        score_fixed: SCALE,
        uncertainty_fixed: 0,
        support: 3,
        timestamp: 11,
        provenance: RelationProvenance {
            origin: 11,
            source: ProvenanceSource::Local,
            revision: 2,
        },
        origin_object_id: 10,
    };

    // Direct structural admission test avoids dependence on canonical queues.
    let mut state = WorkspaceState::empty();
    let attention_item = from_attention(&attention);
    let candidate_item = from_candidate(&candidate);

    if !admit(&mut state, attention_item) || !admit(&mut state, candidate_item) {
        return false;
    }

    if state.item_used != 2 {
        return false;
    }

    let selected = focus(&state);
    if selected.source != WorkspaceSource::Subconscious
        || selected.current_state != super::semantic::FAILED
    {
        return false;
    }

    // Object identity must remain part of the active-scene identity.
    let other_object = WorkspaceItem {
        object_id: 11,
        ..candidate_item
    };
    if SeenKey::from_item(&candidate_item) == SeenKey::from_item(&other_object) {
        return false;
    }

    // Deterministic tie break: lower object ID wins when all primary metrics tie.
    let mut tie_state = WorkspaceState::empty();
    let mut a = candidate_item;
    let mut b = candidate_item;
    a.object_id = 20;
    b.object_id = 21;
    a.timestamp = 100;
    b.timestamp = 100;
    a.support = 3;
    b.support = 3;
    a.salience_fixed = 5000;
    b.salience_fixed = 5000;
    a.uncertainty_fixed = 5000;
    b.uncertainty_fixed = 5000;

    if !admit(&mut tie_state, b) || !admit(&mut tie_state, a) {
        return false;
    }
    if focus(&tie_state).object_id != 20 {
        return false;
    }

    // Full-capacity competition must preserve the strongest item.
    let mut full = WorkspaceState::empty();
    let mut i = 0usize;
    while i < MAX_WORKSPACE {
        let mut item = candidate_item;
        item.object_id = i as u64 + 100;
        item.salience_fixed = i as u32;
        item.timestamp = i as u64;
        if !admit(&mut full, item) {
            return false;
        }
        i += 1;
    }

    let mut strongest = candidate_item;
    strongest.object_id = 999;
    strongest.salience_fixed = SCALE;
    if !admit(&mut full, strongest) {
        return false;
    }
    if focus(&full).object_id != 999 {
        return false;
    }

    clear();
    true
}
