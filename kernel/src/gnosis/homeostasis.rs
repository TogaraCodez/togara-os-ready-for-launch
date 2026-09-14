//! GNOSIS homeostasis summary.
//!
//! This module provides a bounded, derived, non-destructive observation layer
//! over the existing attention queue. It does not mutate canonical registries,
//! does not consume attention records, and does not make decisions.

use super::attention;
use super::relation::ProvenanceSource;
use super::semantic::{ContextId, ObjectId, SemanticId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HomeostaticSummary {
    /// Number of records currently visible in the attention queue.
    pub attention_count: usize,
    /// Sum of all attention scores currently visible in the queue.
    pub total_score: u64,
    /// Highest attention score currently visible in the queue.
    pub max_score: u16,
    /// Number of records originating locally.
    pub local_count: usize,
    /// Number of records originating remotely.
    pub remote_count: usize,
    /// Number of records recovered from persisted or restored state.
    pub recovered_count: usize,
    /// Number of records whose timestamps are within the existing timer tick window.
    pub recent_count: usize,
    /// Number of records that entered a degraded terminal state.
    pub degraded_count: usize,
}

#[inline]
fn is_degraded_state(state: SemanticId) -> bool {
    matches!(
        state,
        super::semantic::FAILED
            | super::semantic::INVALID
            | super::semantic::UNAUTHORIZED
            | super::semantic::EXPIRED
            | super::semantic::UNAVAILABLE
            | super::semantic::OFFLINE
            | super::semantic::STOPPED
    )
}

#[inline]
fn observe_at(now: u64) -> HomeostaticSummary {
    let recent_window = crate::interrupts::timer::TIMER_FREQUENCY as u64;

    let mut summary = HomeostaticSummary {
        attention_count: 0,
        total_score: 0,
        max_score: 0,
        local_count: 0,
        remote_count: 0,
        recovered_count: 0,
        recent_count: 0,
        degraded_count: 0,
    };

    attention::for_each(|record| {
        summary.attention_count += 1;
        summary.total_score = summary.total_score.saturating_add(u64::from(record.score));

        if record.score > summary.max_score {
            summary.max_score = record.score;
        }

        match record.provenance.source {
            ProvenanceSource::Local => summary.local_count += 1,
            ProvenanceSource::Remote => summary.remote_count += 1,
            ProvenanceSource::Recovered => summary.recovered_count += 1,
        }

        let age = now.saturating_sub(record.timestamp);
        if age <= recent_window {
            summary.recent_count += 1;
        }

        if is_degraded_state(record.new_state) {
            summary.degraded_count += 1;
        }
    });

    summary
}

#[inline]
pub fn observe() -> HomeostaticSummary {
    observe_at(crate::system::state::uptime_ticks())
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
) -> super::delta::SemanticDelta {
    super::delta::SemanticDelta {
        object_id,
        semantic_id,
        previous_state,
        new_state,
        context,
        timestamp,
        provenance: super::relation::RelationProvenance {
            origin,
            source,
            revision,
        },
    }
}

#[inline]
pub fn self_test() -> bool {
    attention::clear();

    let empty = observe();
    if empty.attention_count != 0
        || empty.total_score != 0
        || empty.max_score != 0
        || empty.local_count != 0
        || empty.remote_count != 0
        || empty.recovered_count != 0
        || empty.recent_count != 0
        || empty.degraded_count != 0
    {
        attention::clear();
        return false;
    }

    let now = crate::system::state::uptime_ticks();
    let first = make_delta(
        11,
        super::semantic::AUTHORIZATION,
        super::semantic::READY,
        super::semantic::FAILED,
        1,
        now,
        101,
        ProvenanceSource::Local,
        1,
    );
    let second = make_delta(
        12,
        super::semantic::AUTHORIZATION,
        super::semantic::READY,
        super::semantic::EXPIRED,
        2,
        now,
        102,
        ProvenanceSource::Remote,
        2,
    );
    let third = make_delta(
        13,
        super::semantic::AUTHORIZATION,
        super::semantic::READY,
        super::semantic::OFFLINE,
        3,
        now,
        103,
        ProvenanceSource::Recovered,
        3,
    );

    if attention::push(first).is_err()
        || attention::push(second).is_err()
        || attention::push(third).is_err()
    {
        attention::clear();
        return false;
    }

    if attention::count() != 3 {
        attention::clear();
        return false;
    }

    let observe_now = crate::system::state::uptime_ticks();
    let summary = observe_at(observe_now);
    if summary.attention_count != 3 {
        attention::clear();
        return false;
    }

    let mut expected_total = 0u64;
    let mut expected_max = 0u16;
    attention::for_each(|record| {
        expected_total = expected_total.saturating_add(u64::from(record.score));
        if record.score > expected_max {
            expected_max = record.score;
        }
    });

    if summary.total_score != expected_total {
        attention::clear();
        return false;
    }

    if summary.max_score != expected_max {
        attention::clear();
        return false;
    }

    if summary.local_count != 1 || summary.remote_count != 1 || summary.recovered_count != 1 {
        attention::clear();
        return false;
    }

    let recent_limit = crate::interrupts::timer::TIMER_FREQUENCY as u64;
    let mut recent_expected = 0usize;
    if observe_now.saturating_sub(first.timestamp) <= recent_limit {
        recent_expected += 1;
    }
    if observe_now.saturating_sub(second.timestamp) <= recent_limit {
        recent_expected += 1;
    }
    if observe_now.saturating_sub(third.timestamp) <= recent_limit {
        recent_expected += 1;
    }

    if summary.recent_count != recent_expected {
        attention::clear();
        return false;
    }

    if summary.degraded_count != 3 {
        attention::clear();
        return false;
    }

    if attention::count() != 3 {
        attention::clear();
        return false;
    }

    attention::clear();
    true
}
