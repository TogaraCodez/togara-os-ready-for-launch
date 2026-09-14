//! GNOSIS REL-16 — bounded mesh capability discovery.
//!
//! Discovery records capabilities, not permission. A discovered capability
//! never implies authority to invoke it.

use super::relation::{ProvenanceSource, RelationProvenance};
use spin::Mutex;

pub const MAX_NODES: usize = 16;
pub const MAX_CAPABILITIES_PER_NODE: usize = 8;
pub const MAX_WORK: usize = 512;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapabilityRecord {
    pub capability: u32,
    pub quality_fixed: u32,
    pub latency_ticks: u64,
    pub resource_units: u32,
}

impl CapabilityRecord {
    pub const fn empty() -> Self {
        Self {
            capability: 0,
            quality_fixed: 0,
            latency_ticks: 0,
            resource_units: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeshNode {
    pub node_id: u64,
    pub revision: u64,
    pub trust_hint_fixed: u32,
    pub provenance: RelationProvenance,
    pub capabilities: [CapabilityRecord; MAX_CAPABILITIES_PER_NODE],
    pub capability_used: usize,
}

impl MeshNode {
    pub const fn empty() -> Self {
        Self {
            node_id: 0,
            revision: 0,
            trust_hint_fixed: 0,
            provenance: RelationProvenance {
                origin: 0,
                source: ProvenanceSource::Local,
                revision: 0,
            },
            capabilities: [CapabilityRecord::empty(); MAX_CAPABILITIES_PER_NODE],
            capability_used: 0,
        }
    }
}

#[derive(Clone, Copy)]
struct MeshState {
    nodes: [MeshNode; MAX_NODES],
    used: usize,
}

impl MeshState {
    const fn empty() -> Self {
        Self {
            nodes: [MeshNode::empty(); MAX_NODES],
            used: 0,
        }
    }
}

static MESH: Mutex<MeshState> = Mutex::new(MeshState::empty());

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoverError {
    Invalid,
    Full,
    CapabilityFull,
}

#[inline]
pub fn upsert(node: MeshNode) -> Result<(), DiscoverError> {
    if node.node_id == 0 || node.revision == 0 || node.provenance.revision == 0 {
        return Err(DiscoverError::Invalid);
    }

    let mut state = MESH.lock();

    let mut index = 0usize;
    while index < state.used {
        if state.nodes[index].node_id == node.node_id {
            state.nodes[index] = node;
            return Ok(());
        }
        index += 1;
    }

    if state.used >= MAX_NODES {
        return Err(DiscoverError::Full);
    }

    let insert_index = state.used;
    state.nodes[insert_index] = node;
    state.used += 1;
    Ok(())
}

#[inline]
pub fn advertise_capability(
    node_id: u64,
    capability: CapabilityRecord,
) -> Result<(), DiscoverError> {
    if node_id == 0 || capability.capability == 0 {
        return Err(DiscoverError::Invalid);
    }

    let mut state = MESH.lock();
    let mut index = 0usize;

    while index < state.used {
        if state.nodes[index].node_id == node_id {
            let node = &mut state.nodes[index];

            let mut cap = 0usize;
            while cap < node.capability_used {
                if node.capabilities[cap].capability == capability.capability {
                    node.capabilities[cap] = capability;
                    return Ok(());
                }
                cap += 1;
            }

            if node.capability_used >= MAX_CAPABILITIES_PER_NODE {
                return Err(DiscoverError::CapabilityFull);
            }

            node.capabilities[node.capability_used] = capability;
            node.capability_used += 1;
            return Ok(());
        }

        index += 1;
    }

    Err(DiscoverError::Invalid)
}

#[inline]
pub fn count_nodes() -> usize {
    MESH.lock().used
}

#[inline]
pub fn find_capability(capability: u32) -> Option<MeshNode> {
    if capability == 0 {
        return None;
    }

    let state = MESH.lock();
    let mut index = 0usize;

    while index < state.used {
        let node = state.nodes[index];
        let mut cap = 0usize;

        while cap < node.capability_used {
            if node.capabilities[cap].capability == capability {
                return Some(node);
            }
            cap += 1;
        }

        index += 1;
    }

    None
}

#[inline]
pub fn clear() {
    *MESH.lock() = MeshState::empty();
}

#[inline]
pub fn self_test() -> bool {
    clear();

    let provenance = RelationProvenance {
        origin: 1,
        source: ProvenanceSource::Local,
        revision: 1,
    };

    let node = MeshNode {
        node_id: 7,
        revision: 1,
        trust_hint_fixed: 8_000,
        provenance,
        capabilities: [CapabilityRecord::empty(); MAX_CAPABILITIES_PER_NODE],
        capability_used: 0,
    };

    if upsert(node).is_err() {
        clear();
        return false;
    }

    if advertise_capability(
        7,
        CapabilityRecord {
            capability: 100,
            quality_fixed: 9_000,
            latency_ticks: 4,
            resource_units: 8,
        },
    )
    .is_err()
    {
        clear();
        return false;
    }

    let found = find_capability(100);
    let ok = found.is_some() && count_nodes() == 1;
    clear();
    ok
}
