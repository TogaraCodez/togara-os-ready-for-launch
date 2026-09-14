pub mod attention;
pub mod body;
pub mod conscious;
pub mod context;
pub mod delta;
pub mod graph;
pub mod homeostasis;
pub mod knowledge;
pub mod messaging;
pub mod objects;
pub mod query;
pub mod reflex;
pub mod relation;
pub mod semantic;
pub mod soul;
pub mod speculation;
pub mod spirit;
pub mod subconscious;

pub mod compute_fabric;
pub mod decision;
pub mod experience;
pub mod local_compute;
pub mod mesh;
pub mod proof_security;
pub mod reason;
pub mod semantic_abi;
pub mod trust;
pub mod wisdom;

pub mod orchestration;
pub mod transition;

pub const NAME: &[u8] = b"GNOSIS";
pub const VERSION_MAJOR: u64 = 0;
pub const VERSION_MINOR: u64 = 2;

#[inline]
pub fn initialized() -> bool {
    context::is_active()
}

/// Aggregate self-test for the entire GNOSIS subsystem.
pub fn self_test() -> bool {
    let mut ok = true;

    // Core GNOSIS modules
    ok = objects::self_test() && ok;
    ok = delta::self_test() && ok;
    ok = subconscious::self_test() && ok;
    ok = attention::self_test() && ok;
    ok = conscious::self_test() && ok;
    ok = transition::self_test() && ok;
    ok = orchestration::self_test() && ok;
    ok = speculation::self_test() && ok;
    ok = local_compute::self_test() && ok;
    ok = experience::self_test() && ok;
    ok = compute_fabric::self_test() && ok;
    ok = semantic_abi::self_test() && ok;
    ok = mesh::self_test() && ok;
    ok = trust::self_test() && ok;

    // New cognitive stack
    ok = reason::self_test() && ok;
    ok = wisdom::self_test() && ok;
    ok = decision::self_test() && ok;
    ok = proof_security::self_test() && ok;

    ok
}