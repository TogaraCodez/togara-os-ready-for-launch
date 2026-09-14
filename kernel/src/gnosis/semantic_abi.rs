//! GNOSIS REL-15 — Semantic ABI contracts.
//!
//! This is a stable, fixed-layout semantic request/result boundary. It does
//! not perform transport or execution; it defines the information required
//! for components to exchange meaning safely.

use super::relation::{ProvenanceSource, RelationProvenance};
use super::semantic::{ContextId, ObjectId, SemanticId};

pub const ABI_MAJOR: u16 = 1;
pub const ABI_MINOR: u16 = 0;
pub const MAX_PAYLOAD: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbiOperation {
    Observe = 1,
    Predict = 2,
    Speculate = 3,
    Compute = 4,
    Discover = 5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SemanticAbiRequest {
    pub abi_major: u16,
    pub abi_minor: u16,
    pub operation: AbiOperation,
    pub object_id: ObjectId,
    pub semantic_id: SemanticId,
    pub context: ContextId,
    pub payload: [u32; MAX_PAYLOAD],
    pub payload_len: usize,
    pub capability: u32,
    pub authority: u32,
    pub provenance: RelationProvenance,
}

impl SemanticAbiRequest {
    pub const fn empty() -> Self {
        Self {
            abi_major: ABI_MAJOR,
            abi_minor: ABI_MINOR,
            operation: AbiOperation::Observe,
            object_id: 0,
            semantic_id: 0,
            context: 0,
            payload: [0; MAX_PAYLOAD],
            payload_len: 0,
            capability: 0,
            authority: 0,
            provenance: RelationProvenance {
                origin: 0,
                source: ProvenanceSource::Local,
                revision: 0,
            },
        }
    }

    #[inline]
    pub fn valid(&self) -> bool {
        self.abi_major == ABI_MAJOR
            && self.payload_len <= MAX_PAYLOAD
            && self.object_id != 0
            && self.semantic_id != 0
            && self.provenance.revision != 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbiStatus {
    Accepted,
    InvalidVersion,
    InvalidRequest,
    Unauthorized,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SemanticAbiResult {
    pub status: AbiStatus,
    pub operation: AbiOperation,
    pub object_id: ObjectId,
    pub semantic_id: SemanticId,
    pub context: ContextId,
    pub payload: [u32; MAX_PAYLOAD],
    pub payload_len: usize,
    pub provenance: RelationProvenance,
}

impl SemanticAbiResult {
    pub const fn rejected(operation: AbiOperation, status: AbiStatus) -> Self {
        Self {
            status,
            operation,
            object_id: 0,
            semantic_id: 0,
            context: 0,
            payload: [0; MAX_PAYLOAD],
            payload_len: 0,
            provenance: RelationProvenance {
                origin: 0,
                source: ProvenanceSource::Local,
                revision: 0,
            },
        }
    }
}

#[inline]
pub fn validate(request: &SemanticAbiRequest) -> AbiStatus {
    if request.abi_major != ABI_MAJOR {
        return AbiStatus::InvalidVersion;
    }
    if !request.valid() {
        return AbiStatus::InvalidRequest;
    }
    if request.authority == 0 && request.operation == AbiOperation::Compute {
        return AbiStatus::Unauthorized;
    }
    AbiStatus::Accepted
}

#[inline]
pub fn accept(request: SemanticAbiRequest) -> SemanticAbiResult {
    let status = validate(&request);
    if status != AbiStatus::Accepted {
        return SemanticAbiResult::rejected(request.operation, status);
    }

    SemanticAbiResult {
        status,
        operation: request.operation,
        object_id: request.object_id,
        semantic_id: request.semantic_id,
        context: request.context,
        payload: request.payload,
        payload_len: request.payload_len,
        provenance: request.provenance,
    }
}

#[inline]
pub fn self_test() -> bool {
    let mut request = SemanticAbiRequest::empty();
    request.object_id = 1;
    request.semantic_id = 2;
    request.context = 3;
    request.payload[0] = 42;
    request.payload_len = 1;
    request.provenance = RelationProvenance {
        origin: 1,
        source: ProvenanceSource::Local,
        revision: 1,
    };

    if validate(&request) != AbiStatus::Accepted {
        return false;
    }

    let result = accept(request);
    result.status == AbiStatus::Accepted && result.payload[0] == 42
}
