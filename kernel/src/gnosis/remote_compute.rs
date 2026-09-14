//! GNOSIS Remote + Split Compute boundary.
//!
//! Discovery, trust, authority, and local fallback are explicit. No network
//! transport is fabricated here; remote execution is represented as a bounded
//! authorized route for the future transport layer.

use super::compute_fabric::ComputeLocation;
use super::local_compute::LocalComputeResult;
use super::mesh;
use super::relations::{ProvenanceSource, RelationProvenance};
use super::trust::{self, AuthorizationDecision};

pub const MAX_WORK: usize = 512;
pub const SCALE: u32 = 10_000;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    Local,
    Remote,
    Split,
    Defer,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RemoteRequest {
    pub principal_id: u64,
    pub capability: u32,
    pub authority: u32,
    pub minimum_trust_fixed: u32,
    pub estimated_work: u32,
    pub local_quality_fixed: u32,
    pub remote_quality_fixed: u32,
    pub remote_latency_ticks: u64,
    pub max_latency_ticks: u64,
    pub allow_split: bool,
    pub provenance: RelationProvenance,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RemoteDecision {
    pub route: Route,
    pub authorized: bool,
    pub quality_fixed: u32,
    pub local_share_fixed: u32,
    pub remote_share_fixed: u32,
    pub reason_code: u8,
    pub provenance: RelationProvenance,
}
impl RemoteDecision {
    pub const fn defer() -> Self {
        Self {
            route: Route::Defer,
            authorized: false,
            quality_fixed: 0,
            local_share_fixed: 0,
            remote_share_fixed: 0,
            reason_code: 0,
            provenance: RelationProvenance {
                origin: 0,
                source: ProvenanceSource::Local,
                revision: 0,
            },
        }
    }
}
fn valid_p(p: &RelationProvenance) -> bool {
    p.revision != 0
        && matches!(
            p.source,
            ProvenanceSource::Local | ProvenanceSource::Remote | ProvenanceSource::Recovered
        )
}
pub fn arbitrate(r: RemoteRequest, local: Option<LocalComputeResult>) -> RemoteDecision {
    let mut d = RemoteDecision::defer();
    d.provenance = r.provenance;
    if !valid_p(&r.provenance) || r.estimated_work == 0 {
        return d;
    }
    let node = mesh::find_capability(r.capability);
    let auth = trust::authorize(
        trust::AuthorizationRequest {
            principal_id: r.principal_id,
            required_capability: r.capability,
            required_authority: r.authority,
            minimum_trust_fixed: r.minimum_trust_fixed,
        },
        node,
    );
    if auth != AuthorizationDecision::Authorized {
        return d;
    }
    if r.max_latency_ticks != 0 && r.remote_latency_ticks > r.max_latency_ticks {
        return d;
    }
    let local_ok = local.map_or(false, |x| {
        x.disposition == super::local_compute::ComputeDisposition::Evaluated
    });
    if r.remote_quality_fixed >= r.local_quality_fixed && r.remote_quality_fixed > 0 {
        d.route = Route::Remote;
        d.quality_fixed = r.remote_quality_fixed.min(SCALE);
        d.remote_share_fixed = SCALE;
        d.authorized = true;
        d.reason_code = 1
    } else if r.allow_split && local_ok && r.remote_quality_fixed > 0 {
        d.route = Route::Split;
        d.quality_fixed = r.local_quality_fixed.max(r.remote_quality_fixed).min(SCALE);
        d.local_share_fixed = 5_000;
        d.remote_share_fixed = 5_000;
        d.authorized = true;
        d.reason_code = 2
    } else if local_ok {
        d.route = Route::Local;
        d.quality_fixed = r.local_quality_fixed.min(SCALE);
        d.local_share_fixed = SCALE;
        d.reason_code = 3
    } else {
        d.reason_code = 4
    }
    d
}
pub fn fallback(local: Option<LocalComputeResult>, decision: RemoteDecision) -> ComputeLocation {
    if decision.authorized {
        match decision.route {
            Route::Local | Route::Split => ComputeLocation::Local,
            Route::Remote => ComputeLocation::Defer,
            Route::Defer => ComputeLocation::Defer,
        }
    } else if local.is_some() {
        ComputeLocation::Local
    } else {
        ComputeLocation::Defer
    }
}
pub fn self_test() -> bool {
    mesh::clear();
    trust::clear();
    let p = RelationProvenance {
        origin: 1,
        source: ProvenanceSource::Local,
        revision: 1,
    };
    let mut n = mesh::MeshNode::empty();
    n.node_id = 9;
    n.revision = 1;
    n.provenance = p;
    n.capabilities[0] = mesh::CapabilityRecord {
        capability: 77,
        quality_fixed: 9000,
        latency_ticks: 2,
        resource_units: 8,
    };
    n.capability_used = 1;
    if mesh::upsert(n).is_err() {
        return false;
    }
    if !trust::upsert(trust::TrustPrincipal {
        principal_id: 7,
        trust_fixed: 9000,
        authority_mask: 1,
        revision: 1,
        provenance: p,
    }) {
        return false;
    }
    let r = arbitrate(
        RemoteRequest {
            principal_id: 7,
            capability: 77,
            authority: 1,
            minimum_trust_fixed: 8000,
            estimated_work: 8,
            local_quality_fixed: 5000,
            remote_quality_fixed: 9000,
            remote_latency_ticks: 2,
            max_latency_ticks: 10,
            allow_split: true,
            provenance: p,
        },
        None,
    );
    let ok = r.route == Route::Remote && r.authorized;
    mesh::clear();
    trust::clear();
    ok
}
