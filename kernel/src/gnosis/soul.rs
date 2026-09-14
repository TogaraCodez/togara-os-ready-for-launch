//! GNOSIS semantic soul snapshot.
//!
//! REL-07 implements the smallest defensible continuity/self-state descriptor.
//! It is intentionally derived, bounded, and runtime-only. The canonical
//! runtime truth remains in the existing GNOSIS state sources and the kernel
//! system state; Soul only reflects that state in a compact, fixed-size record.
//!
//! This module does not introduce persistence, policy, autonomy, action
//! selection, or a second source of truth. It is a read-only projection of the
//! current kernel self-state for the active runtime instance only.

use super::body::{self, BodySnapshot};
use super::context;
use super::homeostasis::{self, HomeostaticSummary};
use super::reflex::{self, ReflexTrigger};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SoulSnapshot {
    pub kernel_name: [u8; 8],
    pub kernel_version_major: u64,
    pub kernel_version_minor: u64,
    pub active: bool,
    pub generation: u64,
    pub uptime_ticks: u64,
    pub uptime_seconds: u64,
    pub uptime_millis: u64,
    pub keyboard_events: u64,
    pub body: BodySnapshot,
    pub homeostasis: HomeostaticSummary,
    pub reflex: Option<ReflexTrigger>,
}

pub const CAPACITY: usize = 1;

#[inline]
pub fn snapshot() -> SoulSnapshot {
    let kernel_name = {
        let mut name = [0u8; 8];
        let bytes = super::NAME;
        let mut index = 0usize;
        while index < bytes.len() && index < name.len() {
            name[index] = bytes[index];
            index += 1;
        }
        name
    };

    SoulSnapshot {
        kernel_name,
        kernel_version_major: super::VERSION_MAJOR,
        kernel_version_minor: super::VERSION_MINOR,
        active: context::is_active(),
        generation: context::generation(),
        uptime_ticks: context::uptime_ticks(),
        uptime_seconds: context::uptime_seconds(),
        uptime_millis: context::uptime_millis(),
        keyboard_events: context::keyboard_events(),
        body: body::snapshot(),
        homeostasis: homeostasis::observe(),
        reflex: reflex::observe(),
    }
}

#[inline]
pub fn observe() -> SoulSnapshot {
    snapshot()
}

#[inline]
pub fn self_test() -> bool {
    if CAPACITY != 1 {
        return false;
    }

    let delta_before = super::delta::count();
    let attention_before = super::attention::count();
    let knowledge_before = super::knowledge::entry_count();
    let relations_before = super::relation::count();

    // BIOS/QEMU is live while GTEST executes, so timer interrupts may advance
    // between reads. Validate monotonicity instead of requiring an interrupt-
    // free critical section.
    let ticks_before = context::uptime_ticks();
    let seconds_before = context::uptime_seconds();
    let millis_before = context::uptime_millis();
    let generation_before = context::generation();
    let keyboard_before = context::keyboard_events();

    let observed = snapshot();

    let ticks_after = context::uptime_ticks();
    let seconds_after = context::uptime_seconds();
    let millis_after = context::uptime_millis();
    let generation_after = context::generation();
    let keyboard_after = context::keyboard_events();

    if &observed.kernel_name[..super::NAME.len()] != &super::NAME[..] {
        return false;
    }

    if observed.kernel_version_major != super::VERSION_MAJOR
        || observed.kernel_version_minor != super::VERSION_MINOR
    {
        return false;
    }

    if observed.active != context::is_active() {
        return false;
    }

    if observed.generation < generation_before || observed.generation > generation_after {
        return false;
    }

    if observed.keyboard_events < keyboard_before || observed.keyboard_events > keyboard_after {
        return false;
    }

    if observed.uptime_ticks < ticks_before || observed.uptime_ticks > ticks_after {
        return false;
    }

    if observed.uptime_seconds < seconds_before || observed.uptime_seconds > seconds_after {
        return false;
    }

    if observed.uptime_seconds == seconds_before && observed.uptime_millis < millis_before {
        return false;
    }

    if observed.uptime_seconds == seconds_after && observed.uptime_millis > millis_after {
        return false;
    }

    // Body is derived from the same runtime sources. Its timing fields may
    // advance between reads; stable fields must remain identical.
    let current_body = body::snapshot();

    if observed.body.keyboard_events < observed.keyboard_events
        || observed.body.keyboard_events > keyboard_after
        || observed.body.uptime_ticks < observed.uptime_ticks
        || observed.body.uptime_ticks > ticks_after
        || observed.body.uptime_seconds < observed.uptime_seconds
        || observed.body.uptime_seconds > seconds_after
        || observed.body.timer_frequency != current_body.timer_frequency
        || observed.body.total_memory_bytes != current_body.total_memory_bytes
        || observed.body.usable_memory_bytes != current_body.usable_memory_bytes
        || observed.body.reserved_memory_bytes != current_body.reserved_memory_bytes
        || observed.body.allocated_memory_bytes != current_body.allocated_memory_bytes
        || observed.body.reclaimed_memory_bytes != current_body.reclaimed_memory_bytes
        || observed.body.messaging_count != current_body.messaging_count
    {
        return false;
    }

    // Homeostasis is non-destructive. recent_count is time-sensitive, so it
    // may legitimately change if the recent-window boundary is crossed.
    let current_homeostasis = homeostasis::observe();

    if observed.homeostasis.attention_count != current_homeostasis.attention_count
        || observed.homeostasis.total_score != current_homeostasis.total_score
        || observed.homeostasis.max_score != current_homeostasis.max_score
        || observed.homeostasis.local_count != current_homeostasis.local_count
        || observed.homeostasis.remote_count != current_homeostasis.remote_count
        || observed.homeostasis.recovered_count != current_homeostasis.recovered_count
        || observed.homeostasis.degraded_count != current_homeostasis.degraded_count
        || observed.homeostasis.recent_count > observed.homeostasis.attention_count
        || current_homeostasis.recent_count > current_homeostasis.attention_count
    {
        return false;
    }

    if observed.reflex != reflex::observe() {
        return false;
    }

    if super::delta::count() != delta_before
        || super::attention::count() != attention_before
        || super::knowledge::entry_count() != knowledge_before
        || super::relation::count() != relations_before
    {
        return false;
    }

    let repeated = snapshot();

    if repeated.active != observed.active
        || repeated.kernel_name != observed.kernel_name
        || repeated.kernel_version_major != observed.kernel_version_major
        || repeated.kernel_version_minor != observed.kernel_version_minor
        || repeated.reflex != observed.reflex
        || repeated.body.timer_frequency != observed.body.timer_frequency
        || repeated.body.total_memory_bytes != observed.body.total_memory_bytes
        || repeated.body.usable_memory_bytes != observed.body.usable_memory_bytes
        || repeated.body.reserved_memory_bytes != observed.body.reserved_memory_bytes
        || repeated.body.allocated_memory_bytes != observed.body.allocated_memory_bytes
        || repeated.body.reclaimed_memory_bytes != observed.body.reclaimed_memory_bytes
        || repeated.body.messaging_count != observed.body.messaging_count
        || repeated.homeostasis.attention_count != observed.homeostasis.attention_count
        || repeated.homeostasis.total_score != observed.homeostasis.total_score
        || repeated.homeostasis.max_score != observed.homeostasis.max_score
        || repeated.homeostasis.local_count != observed.homeostasis.local_count
        || repeated.homeostasis.remote_count != observed.homeostasis.remote_count
        || repeated.homeostasis.recovered_count != observed.homeostasis.recovered_count
        || repeated.homeostasis.degraded_count != observed.homeostasis.degraded_count
    {
        return false;
    }

    if super::delta::count() != delta_before
        || super::attention::count() != attention_before
        || super::knowledge::entry_count() != knowledge_before
        || super::relation::count() != relations_before
    {
        return false;
    }

    let size = core::mem::size_of::<SoulSnapshot>();
    if size == 0 || size > 512 {
        return false;
    }

    true
}
