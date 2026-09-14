//! GNOSIS Minimal-Knowledge Contracts (MKC).
//!
//! Discloses only the fields explicitly requested by an authorized contract.
//! This is minimal necessary disclosure, not a claim of zero-knowledge proof.

use super::proof_security::DecisionProof;
use super::relations::{ProvenanceSource, RelationProvenance};

pub const MAX_DISCLOSURE_WORDS: usize = 8;
pub const GOAL: u32 = 1 << 0;
pub const EVIDENCE: u32 = 1 << 1;
pub const INFERENCE: u32 = 1 << 2;
pub const ACTION: u32 = 1 << 3;
pub const OUTCOME: u32 = 1 << 4;
pub const CONFIDENCE: u32 = 1 << 5;
pub const COST: u32 = 1 << 6;
pub const PROVENANCE: u32 = 1 << 7;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisclosureRequest {
    pub allowed_mask: u32,
    pub requested_mask: u32,
    pub contract_id: u64,
    pub principal_id: u64,
    pub provenance: RelationProvenance,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Disclosure {
    pub values: [u32; MAX_DISCLOSURE_WORDS],
    pub used: usize,
    pub mask: u32,
    pub contract_id: u64,
    pub provenance: RelationProvenance,
}
impl Disclosure {
    pub const fn empty() -> Self {
        Self {
            values: [0; MAX_DISCLOSURE_WORDS],
            used: 0,
            mask: 0,
            contract_id: 0,
            provenance: RelationProvenance {
                origin: 0,
                source: ProvenanceSource::Local,
                revision: 0,
            },
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MkcStatus {
    Granted,
    Denied,
    Invalid,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MkcResult {
    pub status: MkcStatus,
    pub disclosure: Disclosure,
    pub denied_mask: u32,
}
fn valid_p(p: &RelationProvenance) -> bool {
    p.revision != 0
        && matches!(
            p.source,
            ProvenanceSource::Local | ProvenanceSource::Remote | ProvenanceSource::Recovered
        )
}
pub fn disclose(request: DisclosureRequest, proof: DecisionProof) -> MkcResult {
    let mut out = MkcResult {
        status: MkcStatus::Invalid,
        disclosure: Disclosure::empty(),
        denied_mask: 0,
    };
    if request.contract_id == 0
        || request.principal_id == 0
        || !valid_p(&request.provenance)
        || !valid_p(&proof.provenance)
    {
        return out;
    }
    let granted = request.requested_mask & request.allowed_mask;
    out.denied_mask = request.requested_mask & !request.allowed_mask;
    out.disclosure.contract_id = request.contract_id;
    out.disclosure.provenance = request.provenance;
    out.disclosure.mask = granted;
    let mut put = |bit: u32, value: u32| {
        if granted & bit != 0 && out.disclosure.used < MAX_DISCLOSURE_WORDS {
            out.disclosure.values[out.disclosure.used] = value;
            out.disclosure.used += 1
        }
    };
    put(GOAL, proof.goal);
    put(EVIDENCE, proof.evidence);
    put(INFERENCE, proof.inference);
    put(ACTION, proof.selected_action);
    put(OUTCOME, proof.expected_outcome);
    put(CONFIDENCE, proof.confidence_fixed);
    put(COST, proof.cost_fixed);
    put(PROVENANCE, proof.provenance.revision as u32);
    out.status = if out.denied_mask == 0 {
        MkcStatus::Granted
    } else {
        MkcStatus::Denied
    };
    out
}
pub fn self_test() -> bool {
    let p = RelationProvenance {
        origin: 1,
        source: ProvenanceSource::Local,
        revision: 9,
    };
    let proof = DecisionProof {
        goal: super::semantic::COMPLETE,
        evidence: super::semantic::EVIDENCE,
        inference: super::semantic::INFERENCE,
        constraint_mask: 1,
        alternatives: [super::semantic::EXECUTE; 8],
        alternative_count: 1,
        selected_action: super::semantic::EXECUTE,
        confidence_fixed: 9000,
        cost_fixed: 4,
        expected_outcome: super::semantic::COMPLETE,
        rationale: super::semantic::INFERENCE,
        provenance: p,
    };
    let r = disclose(
        DisclosureRequest {
            allowed_mask: GOAL | ACTION,
            requested_mask: GOAL | ACTION | COST,
            contract_id: 1,
            principal_id: 1,
            provenance: p,
        },
        proof,
    );
    r.status == MkcStatus::Denied && r.disclosure.used == 2 && r.denied_mask == COST
}
