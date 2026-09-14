//! GNOSIS Semantic Body Compiler.
//!
//! Compiles an accepted Semantic ABI intent into a bounded embodiment-neutral
//! action plan. Hardware-specific execution remains outside this module.

use super::relations::{ProvenanceSource, RelationProvenance};
use super::semantic::{ContextId, ObjectId, SemanticId};
use super::semantic_abi::{self, AbiOperation, AbiStatus, SemanticAbiRequest, SemanticAbiResult};

pub const MAX_ACTIONS: usize = 8;
pub const MAX_WORK: usize = 128;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyAction {
    pub action: SemanticId,
    pub object_id: ObjectId,
    pub context: ContextId,
    pub parameter: u32,
}
impl BodyAction {
    pub const fn empty() -> Self {
        Self {
            action: 0,
            object_id: 0,
            context: 0,
            parameter: 0,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyPlan {
    pub accepted: bool,
    pub source_operation: AbiOperation,
    pub actions: [BodyAction; MAX_ACTIONS],
    pub action_count: usize,
    pub work_used: usize,
    pub provenance: RelationProvenance,
}
impl BodyPlan {
    pub const fn empty() -> Self {
        Self {
            accepted: false,
            source_operation: AbiOperation::Observe,
            actions: [BodyAction::empty(); MAX_ACTIONS],
            action_count: 0,
            work_used: 0,
            provenance: RelationProvenance {
                origin: 0,
                source: ProvenanceSource::Local,
                revision: 0,
            },
        }
    }
}
pub fn compile(request: SemanticAbiRequest) -> BodyPlan {
    let accepted = semantic_abi::accept(request);
    compile_result(accepted)
}
pub fn compile_result(result: SemanticAbiResult) -> BodyPlan {
    let mut p = BodyPlan::empty();
    p.source_operation = result.operation;
    p.provenance = result.provenance;
    if result.status != AbiStatus::Accepted || result.object_id == 0 || result.semantic_id == 0 {
        return p;
    }
    if !matches!(
        result.operation,
        AbiOperation::Compute | AbiOperation::Speculate | AbiOperation::Predict
    ) {
        return p;
    }
    p.actions[0] = BodyAction {
        action: result.semantic_id,
        object_id: result.object_id,
        context: result.context,
        parameter: if result.payload_len > 0 {
            result.payload[0]
        } else {
            0
        },
    };
    p.action_count = 1;
    p.work_used = 1;
    p.accepted = true;
    p
}
pub fn self_test() -> bool {
    let p = RelationProvenance {
        origin: 1,
        source: ProvenanceSource::Local,
        revision: 1,
    };
    let mut r = SemanticAbiRequest::empty();
    r.operation = AbiOperation::Compute;
    r.object_id = 1;
    r.semantic_id = super::semantic::EXECUTE;
    r.context = 2;
    r.payload[0] = 42;
    r.payload_len = 1;
    r.authority = 1;
    r.provenance = p;
    let b = compile(r);
    b.accepted && b.action_count == 1 && b.actions[0].parameter == 42
}
