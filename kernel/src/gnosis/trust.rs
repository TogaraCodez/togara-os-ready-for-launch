//! GNOSIS REL-17 — bounded trust and authority integration.
//!
//! Capability answers "can". Authority answers "may". Trust answers "how much
//! should this source be relied upon". None of these mutate canonical state.

use super::mesh::MeshNode;
use super::relation::{ProvenanceSource, RelationProvenance};
use spin::Mutex;

pub const SCALE: u32 = 10_000;
pub const MAX_PRINCIPALS: usize = 16;
pub const MAX_WORK: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrustPrincipal {
    pub principal_id: u64,
    pub trust_fixed: u32,
    pub authority_mask: u32,
    pub revision: u64,
    pub provenance: RelationProvenance,
}

impl TrustPrincipal {
    pub const fn empty() -> Self {
        Self {
            principal_id: 0,
            trust_fixed: 0,
            authority_mask: 0,
            revision: 0,
            provenance: RelationProvenance {
                origin: 0,
                source: ProvenanceSource::Local,
                revision: 0,
            },
        }
    }
}

#[derive(Clone, Copy)]
struct TrustState {
    principals: [TrustPrincipal; MAX_PRINCIPALS],
    used: usize,
}

impl TrustState {
    const fn empty() -> Self {
        Self {
            principals: [TrustPrincipal::empty(); MAX_PRINCIPALS],
            used: 0,
        }
    }
}

static TRUST: Mutex<TrustState> = Mutex::new(TrustState::empty());

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuthorizationRequest {
    pub principal_id: u64,
    pub required_capability: u32,
    pub required_authority: u32,
    pub minimum_trust_fixed: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorizationDecision {
    Authorized,
    NoPrincipal,
    MissingCapability,
    MissingAuthority,
    InsufficientTrust,
}

#[inline]
pub fn upsert(principal: TrustPrincipal) -> bool {
    if principal.principal_id == 0 || principal.revision == 0 || principal.provenance.revision == 0
    {
        return false;
    }

    let mut state = TRUST.lock();

    let mut index = 0usize;
    while index < state.used {
        if state.principals[index].principal_id == principal.principal_id {
            state.principals[index] = principal;
            return true;
        }
        index += 1;
    }

    if state.used >= MAX_PRINCIPALS {
        return false;
    }

    let insert_index = state.used;
    state.principals[insert_index] = principal;
    state.used += 1;
    true
}

#[inline]
pub fn authorize(request: AuthorizationRequest, node: Option<MeshNode>) -> AuthorizationDecision {
    let principal = {
        let state = TRUST.lock();
        let mut found = None;
        let mut index = 0usize;

        while index < state.used {
            if state.principals[index].principal_id == request.principal_id {
                found = Some(state.principals[index]);
                break;
            }
            index += 1;
        }

        found
    };

    let principal = match principal {
        Some(value) => value,
        None => return AuthorizationDecision::NoPrincipal,
    };

    if request.required_capability != 0 {
        let node = match node {
            Some(value) => value,
            None => return AuthorizationDecision::MissingCapability,
        };

        let mut found = false;
        let mut index = 0usize;
        while index < node.capability_used {
            if node.capabilities[index].capability == request.required_capability {
                found = true;
                break;
            }
            index += 1;
        }

        if !found {
            return AuthorizationDecision::MissingCapability;
        }
    }

    if request.required_authority != 0
        && principal.authority_mask & request.required_authority != request.required_authority
    {
        return AuthorizationDecision::MissingAuthority;
    }

    if principal.trust_fixed.min(SCALE) < request.minimum_trust_fixed.min(SCALE) {
        return AuthorizationDecision::InsufficientTrust;
    }

    AuthorizationDecision::Authorized
}

#[inline]
pub fn trust_of(principal_id: u64) -> u32 {
    let state = TRUST.lock();
    let mut index = 0usize;

    while index < state.used {
        if state.principals[index].principal_id == principal_id {
            return state.principals[index].trust_fixed.min(SCALE);
        }
        index += 1;
    }

    0
}

#[inline]
pub fn clear() {
    *TRUST.lock() = TrustState::empty();
}

#[inline]
pub fn self_test() -> bool {
    clear();

    let provenance = RelationProvenance {
        origin: 1,
        source: ProvenanceSource::Local,
        revision: 1,
    };

    if !upsert(TrustPrincipal {
        principal_id: 7,
        trust_fixed: 9_000,
        authority_mask: 0b11,
        revision: 1,
        provenance,
    }) {
        clear();
        return false;
    }

    let mut node = MeshNode::empty();
    node.node_id = 8;
    node.revision = 1;
    node.provenance = provenance;
    node.capabilities[0] = super::mesh::CapabilityRecord {
        capability: 100,
        quality_fixed: 9_000,
        latency_ticks: 4,
        resource_units: 8,
    };
    node.capability_used = 1;

    let decision = authorize(
        AuthorizationRequest {
            principal_id: 7,
            required_capability: 100,
            required_authority: 0b01,
            minimum_trust_fixed: 8_000,
        },
        Some(node),
    );

    let denied = authorize(
        AuthorizationRequest {
            principal_id: 7,
            required_capability: 100,
            required_authority: 0b100,
            minimum_trust_fixed: 8_000,
        },
        Some(node),
    );

    let ok = decision == AuthorizationDecision::Authorized
        && denied == AuthorizationDecision::MissingAuthority
        && trust_of(7) == 9_000;

    clear();
    ok
}
