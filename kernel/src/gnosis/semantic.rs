//! GNOSIS semantic ID registry (canonical identities)
//!
//! Semantic core v0.1 — stable, no_std, explicit ID assignments.

pub type SemanticId = u32;
pub type ObjectId = u64;
pub type ContextId = u64;

// Domain ranges (inclusive)
pub const META_RANGE_START: SemanticId = 0x0000_0001;
pub const META_RANGE_END: SemanticId = 0x0000_00FF;

pub const ENTITY_RANGE_START: SemanticId = 0x0000_0100;
pub const ENTITY_RANGE_END: SemanticId = 0x0000_0FFF;

pub const ACTION_RANGE_START: SemanticId = 0x0000_1000;
pub const ACTION_RANGE_END: SemanticId = 0x0000_1FFF;

pub const STATE_RANGE_START: SemanticId = 0x0000_2000;
pub const STATE_RANGE_END: SemanticId = 0x0000_2FFF;

pub const RELATION_RANGE_START: SemanticId = 0x0000_3000;
pub const RELATION_RANGE_END: SemanticId = 0x0000_3FFF;

pub const LOGIC_RANGE_START: SemanticId = 0x0000_4000;
pub const LOGIC_RANGE_END: SemanticId = 0x0000_4FFF;

pub const TIME_RANGE_START: SemanticId = 0x0000_5000;
pub const TIME_RANGE_END: SemanticId = 0x0000_5FFF;

pub const SPACE_RANGE_START: SemanticId = 0x0000_6000;
pub const SPACE_RANGE_END: SemanticId = 0x0000_6FFF;

pub const COMMUNICATION_RANGE_START: SemanticId = 0x0000_7000;
pub const COMMUNICATION_RANGE_END: SemanticId = 0x0000_7FFF;

pub const NETWORK_RANGE_START: SemanticId = 0x0000_8000;
pub const NETWORK_RANGE_END: SemanticId = 0x0000_8FFF;

pub const SECURITY_RANGE_START: SemanticId = 0x0000_9000;
pub const SECURITY_RANGE_END: SemanticId = 0x0000_9FFF;

pub const COMPUTATION_RANGE_START: SemanticId = 0x0000_A000;
pub const COMPUTATION_RANGE_END: SemanticId = 0x0000_AFFF;

pub const KNOWLEDGE_RANGE_START: SemanticId = 0x0000_B000;
pub const KNOWLEDGE_RANGE_END: SemanticId = 0x0000_BFFF;

pub const VALUE_RANGE_START: SemanticId = 0x0000_C000;
pub const VALUE_RANGE_END: SemanticId = 0x0000_CFFF;

pub const DEVICE_RANGE_START: SemanticId = 0x0000_D000;
pub const DEVICE_RANGE_END: SemanticId = 0x0000_DFFF;

pub const TOGARA_RANGE_START: SemanticId = 0x0000_E000;
pub const TOGARA_RANGE_END: SemanticId = 0x0000_EFFF;

// Canonical semantic primitives (explicit IDs inside ranges)
// META domain (start at META_RANGE_START)
pub const META: SemanticId = META_RANGE_START + 0;
pub const ENTITY: SemanticId = META_RANGE_START + 1;
pub const OBJECT: SemanticId = META_RANGE_START + 2;
pub const CONCEPT: SemanticId = META_RANGE_START + 3;
pub const TYPE: SemanticId = META_RANGE_START + 4;
pub const VALUE: SemanticId = META_RANGE_START + 5;
pub const ATTRIBUTE: SemanticId = META_RANGE_START + 6;
pub const PROPERTY: SemanticId = META_RANGE_START + 7;
pub const IDENTIFIER: SemanticId = META_RANGE_START + 8;
pub const REFERENCE: SemanticId = META_RANGE_START + 9;
pub const RESOURCE: SemanticId = META_RANGE_START + 10;
pub const EVENT: SemanticId = META_RANGE_START + 11;
pub const CONTEXT: SemanticId = META_RANGE_START + 12;
pub const RELATION: SemanticId = META_RANGE_START + 13;

// ENTITY domain (start at ENTITY_RANGE_START)
pub const PERSON: SemanticId = ENTITY_RANGE_START + 0;
pub const USER: SemanticId = ENTITY_RANGE_START + 1;
pub const DEVICE_ENTITY: SemanticId = ENTITY_RANGE_START + 2; // avoid name collision with DEVICE domain
pub const AGENT: SemanticId = ENTITY_RANGE_START + 3;
pub const SERVICE: SemanticId = ENTITY_RANGE_START + 4;
pub const NODE: SemanticId = ENTITY_RANGE_START + 5;
pub const MESSAGE: SemanticId = ENTITY_RANGE_START + 6;
pub const NETWORK_ENTITY: SemanticId = ENTITY_RANGE_START + 7; // avoid collision
pub const PROCESS: SemanticId = ENTITY_RANGE_START + 8;
pub const TASK: SemanticId = ENTITY_RANGE_START + 9;
pub const LOCATION: SemanticId = ENTITY_RANGE_START + 10;
pub const ORGANIZATION: SemanticId = ENTITY_RANGE_START + 11;

// ACTION domain
pub const CREATE: SemanticId = ACTION_RANGE_START + 0;
pub const READ: SemanticId = ACTION_RANGE_START + 1;
pub const WRITE: SemanticId = ACTION_RANGE_START + 2;
pub const UPDATE: SemanticId = ACTION_RANGE_START + 3;
pub const DELETE: SemanticId = ACTION_RANGE_START + 4;
pub const SEND: SemanticId = ACTION_RANGE_START + 5;
pub const RECEIVE: SemanticId = ACTION_RANGE_START + 6;
pub const RELAY: SemanticId = ACTION_RANGE_START + 7;
pub const FORWARD: SemanticId = ACTION_RANGE_START + 8;
pub const CONNECT: SemanticId = ACTION_RANGE_START + 9;
pub const DISCONNECT: SemanticId = ACTION_RANGE_START + 10;
pub const EXECUTE: SemanticId = ACTION_RANGE_START + 11;
pub const VERIFY: SemanticId = ACTION_RANGE_START + 12;
pub const AUTHORIZE: SemanticId = ACTION_RANGE_START + 13;
pub const DENY: SemanticId = ACTION_RANGE_START + 14;
pub const DISCOVER: SemanticId = ACTION_RANGE_START + 15;
pub const STORE: SemanticId = ACTION_RANGE_START + 16;
pub const LOAD: SemanticId = ACTION_RANGE_START + 17;
pub const SAVE: SemanticId = ACTION_RANGE_START + 18;

// STATE domain
pub const NEW: SemanticId = STATE_RANGE_START + 0;
pub const READY: SemanticId = STATE_RANGE_START + 1;
pub const ACTIVE: SemanticId = STATE_RANGE_START + 2;
pub const INACTIVE: SemanticId = STATE_RANGE_START + 3;
pub const RUNNING: SemanticId = STATE_RANGE_START + 4;
pub const STOPPED: SemanticId = STATE_RANGE_START + 5;
pub const PAUSED: SemanticId = STATE_RANGE_START + 6;
pub const PENDING: SemanticId = STATE_RANGE_START + 7;
pub const COMPLETE: SemanticId = STATE_RANGE_START + 8;
pub const FAILED: SemanticId = STATE_RANGE_START + 9;
pub const EXPIRED: SemanticId = STATE_RANGE_START + 10;
pub const AVAILABLE: SemanticId = STATE_RANGE_START + 11;
pub const UNAVAILABLE: SemanticId = STATE_RANGE_START + 12;
pub const ONLINE: SemanticId = STATE_RANGE_START + 13;
pub const OFFLINE: SemanticId = STATE_RANGE_START + 14;
pub const KNOWN: SemanticId = STATE_RANGE_START + 15;
pub const UNKNOWN_STATE: SemanticId = STATE_RANGE_START + 16; // avoid collision with LOGIC.UNKNOWN
pub const VALID: SemanticId = STATE_RANGE_START + 17;
pub const INVALID: SemanticId = STATE_RANGE_START + 18;
pub const VERIFIED: SemanticId = STATE_RANGE_START + 19;
pub const UNVERIFIED: SemanticId = STATE_RANGE_START + 20;
pub const AUTHORIZED: SemanticId = STATE_RANGE_START + 21;
pub const UNAUTHORIZED: SemanticId = STATE_RANGE_START + 22;

// RELATION domain
pub const IS: SemanticId = RELATION_RANGE_START + 0;
pub const IS_A: SemanticId = RELATION_RANGE_START + 1;
pub const HAS: SemanticId = RELATION_RANGE_START + 2;
pub const PART_OF: SemanticId = RELATION_RANGE_START + 3;
pub const MEMBER_OF: SemanticId = RELATION_RANGE_START + 4;
pub const CREATED_BY: SemanticId = RELATION_RANGE_START + 5;
pub const OWNED_BY: SemanticId = RELATION_RANGE_START + 6;
pub const CONTROLLED_BY: SemanticId = RELATION_RANGE_START + 7;
pub const USES: SemanticId = RELATION_RANGE_START + 8;
pub const DEPENDS_ON: SemanticId = RELATION_RANGE_START + 9;
pub const REQUIRES: SemanticId = RELATION_RANGE_START + 10;
pub const PROVIDES: SemanticId = RELATION_RANGE_START + 11;
pub const CONNECTED_TO: SemanticId = RELATION_RANGE_START + 12;
pub const DISCONNECTED_FROM: SemanticId = RELATION_RANGE_START + 13;
pub const SENDS_TO: SemanticId = RELATION_RANGE_START + 14;
pub const RECEIVES_FROM: SemanticId = RELATION_RANGE_START + 15;
pub const ROUTES_TO: SemanticId = RELATION_RANGE_START + 16;
pub const RELAYS_TO: SemanticId = RELATION_RANGE_START + 17;
pub const CAUSES: SemanticId = RELATION_RANGE_START + 18;
pub const PREVENTS: SemanticId = RELATION_RANGE_START + 19;
pub const ALLOWS: SemanticId = RELATION_RANGE_START + 20;
pub const DENIES: SemanticId = RELATION_RANGE_START + 21;
pub const RELATED_TO: SemanticId = RELATION_RANGE_START + 22;

// LOGIC domain
pub const AND: SemanticId = LOGIC_RANGE_START + 0;
pub const OR: SemanticId = LOGIC_RANGE_START + 1;
pub const NOT: SemanticId = LOGIC_RANGE_START + 2;
pub const XOR: SemanticId = LOGIC_RANGE_START + 3;
pub const IF: SemanticId = LOGIC_RANGE_START + 4;
pub const THEN: SemanticId = LOGIC_RANGE_START + 5;
pub const ELSE: SemanticId = LOGIC_RANGE_START + 6;
pub const ANY: SemanticId = LOGIC_RANGE_START + 7;
pub const ALL: SemanticId = LOGIC_RANGE_START + 8;
pub const NONE: SemanticId = LOGIC_RANGE_START + 9;
pub const SOME: SemanticId = LOGIC_RANGE_START + 10;
pub const EQUAL: SemanticId = LOGIC_RANGE_START + 11;
pub const NOT_EQUAL: SemanticId = LOGIC_RANGE_START + 12;
pub const GREATER: SemanticId = LOGIC_RANGE_START + 13;
pub const LESS: SemanticId = LOGIC_RANGE_START + 14;
pub const GREATER_EQUAL: SemanticId = LOGIC_RANGE_START + 15;
pub const LESS_EQUAL: SemanticId = LOGIC_RANGE_START + 16;
pub const TRUE: SemanticId = LOGIC_RANGE_START + 17;
pub const FALSE: SemanticId = LOGIC_RANGE_START + 18;
pub const NULL: SemanticId = LOGIC_RANGE_START + 19;
pub const UNKNOWN: SemanticId = LOGIC_RANGE_START + 20;

// COMMUNICATION domain
pub const PACKET: SemanticId = COMMUNICATION_RANGE_START + 0;
pub const FRAME: SemanticId = COMMUNICATION_RANGE_START + 1;
pub const HEADER: SemanticId = COMMUNICATION_RANGE_START + 2;
pub const PAYLOAD_CONST: SemanticId = COMMUNICATION_RANGE_START + 3; // avoid shadowing VALUE
pub const CHANNEL: SemanticId = COMMUNICATION_RANGE_START + 4;
pub const ENDPOINT: SemanticId = COMMUNICATION_RANGE_START + 5;
pub const ROUTE: SemanticId = COMMUNICATION_RANGE_START + 6;
pub const HOP: SemanticId = COMMUNICATION_RANGE_START + 7;
pub const BROADCAST: SemanticId = COMMUNICATION_RANGE_START + 8;
pub const UNICAST: SemanticId = COMMUNICATION_RANGE_START + 9;
pub const MULTICAST: SemanticId = COMMUNICATION_RANGE_START + 10;
pub const ACK: SemanticId = COMMUNICATION_RANGE_START + 11;
pub const NACK: SemanticId = COMMUNICATION_RANGE_START + 12;
pub const DELIVERY: SemanticId = COMMUNICATION_RANGE_START + 13;
pub const QUEUE: SemanticId = COMMUNICATION_RANGE_START + 14;
pub const TTL: SemanticId = COMMUNICATION_RANGE_START + 15;
pub const SEQUENCE: SemanticId = COMMUNICATION_RANGE_START + 16;

// NETWORK domain
pub const PEER: SemanticId = NETWORK_RANGE_START + 0;
pub const NEIGHBOR: SemanticId = NETWORK_RANGE_START + 1;
pub const MESH: SemanticId = NETWORK_RANGE_START + 2;
pub const CLUSTER: SemanticId = NETWORK_RANGE_START + 3;
pub const GATEWAY: SemanticId = NETWORK_RANGE_START + 4;
pub const ROUTER: SemanticId = NETWORK_RANGE_START + 5;
pub const REPEATER: SemanticId = NETWORK_RANGE_START + 6;
pub const RELAY_NET: SemanticId = NETWORK_RANGE_START + 7; // avoid NAME COLLISION
pub const ADDRESS: SemanticId = NETWORK_RANGE_START + 8;
pub const DISCOVERY: SemanticId = NETWORK_RANGE_START + 9;
pub const BANDWIDTH: SemanticId = NETWORK_RANGE_START + 10;
pub const LATENCY: SemanticId = NETWORK_RANGE_START + 11;
pub const RANGE_CONST: SemanticId = NETWORK_RANGE_START + 12; // avoid conflict with std
pub const SIGNAL: SemanticId = NETWORK_RANGE_START + 13;
pub const DIRECT: SemanticId = NETWORK_RANGE_START + 14;
pub const INDIRECT: SemanticId = NETWORK_RANGE_START + 15;
pub const MULTIHOP: SemanticId = NETWORK_RANGE_START + 16;

// SECURITY domain
pub const SECRET: SemanticId = SECURITY_RANGE_START + 0;
pub const KEY: SemanticId = SECURITY_RANGE_START + 1;
pub const PUBLIC_KEY: SemanticId = SECURITY_RANGE_START + 2;
pub const PRIVATE_KEY: SemanticId = SECURITY_RANGE_START + 3;
pub const CREDENTIAL: SemanticId = SECURITY_RANGE_START + 4;
pub const TOKEN: SemanticId = SECURITY_RANGE_START + 5;
pub const SIGNATURE: SemanticId = SECURITY_RANGE_START + 6;
pub const HASH: SemanticId = SECURITY_RANGE_START + 7;
pub const PROOF: SemanticId = SECURITY_RANGE_START + 8;
pub const CERTIFICATE: SemanticId = SECURITY_RANGE_START + 9;
pub const AUTHENTICATION: SemanticId = SECURITY_RANGE_START + 10;
pub const AUTHORIZATION: SemanticId = SECURITY_RANGE_START + 11;
pub const CAPABILITY: SemanticId = SECURITY_RANGE_START + 12;
pub const PERMISSION: SemanticId = SECURITY_RANGE_START + 13;
pub const POLICY: SemanticId = SECURITY_RANGE_START + 14;
pub const TRUST: SemanticId = SECURITY_RANGE_START + 15;
pub const INTEGRITY: SemanticId = SECURITY_RANGE_START + 16;
pub const CONFIDENTIALITY: SemanticId = SECURITY_RANGE_START + 17;
pub const PRIVACY: SemanticId = SECURITY_RANGE_START + 18;
pub const REVOKE: SemanticId = SECURITY_RANGE_START + 19;
pub const EXPIRE: SemanticId = SECURITY_RANGE_START + 20;

// COMPUTATION domain (reserved, no initial constants specified beyond ranges)

// KNOWLEDGE domain
pub const DATA: SemanticId = KNOWLEDGE_RANGE_START + 0;
pub const INFORMATION: SemanticId = KNOWLEDGE_RANGE_START + 1;
pub const KNOWLEDGE: SemanticId = KNOWLEDGE_RANGE_START + 2;
pub const MEANING: SemanticId = KNOWLEDGE_RANGE_START + 3;
pub const SEMANTIC: SemanticId = KNOWLEDGE_RANGE_START + 4;
pub const FACT: SemanticId = KNOWLEDGE_RANGE_START + 5;
pub const OBSERVATION: SemanticId = KNOWLEDGE_RANGE_START + 6;
pub const CLAIM: SemanticId = KNOWLEDGE_RANGE_START + 7;
pub const EVIDENCE: SemanticId = KNOWLEDGE_RANGE_START + 8;
pub const SOURCE_K: SemanticId = KNOWLEDGE_RANGE_START + 9;
pub const PROVENANCE: SemanticId = KNOWLEDGE_RANGE_START + 10;
pub const MEMORY_K: SemanticId = KNOWLEDGE_RANGE_START + 11;
pub const MODEL: SemanticId = KNOWLEDGE_RANGE_START + 12;
pub const RULE: SemanticId = KNOWLEDGE_RANGE_START + 13;
pub const CONSTRAINT: SemanticId = KNOWLEDGE_RANGE_START + 14;
pub const DEPENDENCY: SemanticId = KNOWLEDGE_RANGE_START + 15;
pub const GRAPH: SemanticId = KNOWLEDGE_RANGE_START + 16;
pub const INDEX: SemanticId = KNOWLEDGE_RANGE_START + 17;
pub const QUERY: SemanticId = KNOWLEDGE_RANGE_START + 18;
pub const ANSWER: SemanticId = KNOWLEDGE_RANGE_START + 19;
pub const RESULT: SemanticId = KNOWLEDGE_RANGE_START + 20;
pub const INFERENCE: SemanticId = KNOWLEDGE_RANGE_START + 21;
pub const PREDICTION: SemanticId = KNOWLEDGE_RANGE_START + 22;
pub const PATTERN: SemanticId = KNOWLEDGE_RANGE_START + 23;
pub const CONFIDENCE: SemanticId = KNOWLEDGE_RANGE_START + 24;
pub const CERTAINTY: SemanticId = KNOWLEDGE_RANGE_START + 25;

// VALUE domain
pub const ASSET: SemanticId = VALUE_RANGE_START + 0;
pub const BALANCE: SemanticId = VALUE_RANGE_START + 1;
pub const SUPPLY: SemanticId = VALUE_RANGE_START + 2;
pub const DEMAND: SemanticId = VALUE_RANGE_START + 3;
pub const PRICE: SemanticId = VALUE_RANGE_START + 4;
pub const COST: SemanticId = VALUE_RANGE_START + 5;
pub const FEE: SemanticId = VALUE_RANGE_START + 6;
pub const REWARD: SemanticId = VALUE_RANGE_START + 7;
pub const CREDIT: SemanticId = VALUE_RANGE_START + 8;
pub const DEBIT: SemanticId = VALUE_RANGE_START + 9;
pub const TRANSFER: SemanticId = VALUE_RANGE_START + 10;
pub const MINT: SemanticId = VALUE_RANGE_START + 11;
pub const BURN: SemanticId = VALUE_RANGE_START + 12;
pub const ISSUE: SemanticId = VALUE_RANGE_START + 13;
pub const REDEEM: SemanticId = VALUE_RANGE_START + 14;
pub const PAYMENT: SemanticId = VALUE_RANGE_START + 15;
pub const SETTLEMENT: SemanticId = VALUE_RANGE_START + 16;
pub const LEDGER: SemanticId = VALUE_RANGE_START + 17;
pub const LICENSE: SemanticId = VALUE_RANGE_START + 18;
pub const QUOTA: SemanticId = VALUE_RANGE_START + 19;
pub const OBLIGATION: SemanticId = VALUE_RANGE_START + 20;
pub const OFFER: SemanticId = VALUE_RANGE_START + 21;
pub const AGREEMENT: SemanticId = VALUE_RANGE_START + 22;

// DEVICE domain
pub const CPU: SemanticId = DEVICE_RANGE_START + 0;
pub const CORE: SemanticId = DEVICE_RANGE_START + 1;
pub const THREAD: SemanticId = DEVICE_RANGE_START + 2;
pub const MEMORY_D: SemanticId = DEVICE_RANGE_START + 3;
pub const PAGE: SemanticId = DEVICE_RANGE_START + 4;
pub const FRAME_D: SemanticId = DEVICE_RANGE_START + 5;
pub const STORAGE: SemanticId = DEVICE_RANGE_START + 6;
pub const FILE: SemanticId = DEVICE_RANGE_START + 7;
pub const BLOCK: SemanticId = DEVICE_RANGE_START + 8;
pub const DRIVER: SemanticId = DEVICE_RANGE_START + 9;
pub const INTERRUPT_D: SemanticId = DEVICE_RANGE_START + 10;
pub const TIMER: SemanticId = DEVICE_RANGE_START + 11;
pub const CLOCK: SemanticId = DEVICE_RANGE_START + 12;
pub const POWER: SemanticId = DEVICE_RANGE_START + 13;
pub const BATTERY: SemanticId = DEVICE_RANGE_START + 14;
pub const SENSOR: SemanticId = DEVICE_RANGE_START + 15;
pub const ACTUATOR: SemanticId = DEVICE_RANGE_START + 16;
pub const CAMERA: SemanticId = DEVICE_RANGE_START + 17;
pub const MICROPHONE: SemanticId = DEVICE_RANGE_START + 18;

// TOGARA domain
pub const KOBJ: SemanticId = TOGARA_RANGE_START + 0;
pub const SEM: SemanticId = TOGARA_RANGE_START + 1;
pub const LINK: SemanticId = TOGARA_RANGE_START + 2;
pub const CTX: SemanticId = TOGARA_RANGE_START + 3;
pub const INTENT: SemanticId = TOGARA_RANGE_START + 4;
pub const FLOW: SemanticId = TOGARA_RANGE_START + 5;
pub const CAP: SemanticId = TOGARA_RANGE_START + 6;
pub const REF: SemanticId = TOGARA_RANGE_START + 8;
pub const STATE_OBJECT: SemanticId = TOGARA_RANGE_START + 9;
pub const ACTION_OBJECT: SemanticId = TOGARA_RANGE_START + 10;
pub const OBS: SemanticId = TOGARA_RANGE_START + 11;
pub const DERIV: SemanticId = TOGARA_RANGE_START + 12;

// Utility API
pub const fn is_valid(id: SemanticId) -> bool {
    (id >= META_RANGE_START && id <= META_RANGE_END)
        || (id >= ENTITY_RANGE_START && id <= ENTITY_RANGE_END)
        || (id >= ACTION_RANGE_START && id <= ACTION_RANGE_END)
        || (id >= STATE_RANGE_START && id <= STATE_RANGE_END)
        || (id >= RELATION_RANGE_START && id <= RELATION_RANGE_END)
        || (id >= LOGIC_RANGE_START && id <= LOGIC_RANGE_END)
        || (id >= TIME_RANGE_START && id <= TIME_RANGE_END)
        || (id >= SPACE_RANGE_START && id <= SPACE_RANGE_END)
        || (id >= COMMUNICATION_RANGE_START && id <= COMMUNICATION_RANGE_END)
        || (id >= NETWORK_RANGE_START && id <= NETWORK_RANGE_END)
        || (id >= SECURITY_RANGE_START && id <= SECURITY_RANGE_END)
        || (id >= COMPUTATION_RANGE_START && id <= COMPUTATION_RANGE_END)
        || (id >= KNOWLEDGE_RANGE_START && id <= KNOWLEDGE_RANGE_END)
        || (id >= VALUE_RANGE_START && id <= VALUE_RANGE_END)
        || (id >= DEVICE_RANGE_START && id <= DEVICE_RANGE_END)
        || (id >= TOGARA_RANGE_START && id <= TOGARA_RANGE_END)
}

pub const fn domain(id: SemanticId) -> SemanticId {
    if id >= META_RANGE_START && id <= META_RANGE_END {
        META_RANGE_START
    } else if id >= ENTITY_RANGE_START && id <= ENTITY_RANGE_END {
        ENTITY_RANGE_START
    } else if id >= ACTION_RANGE_START && id <= ACTION_RANGE_END {
        ACTION_RANGE_START
    } else if id >= STATE_RANGE_START && id <= STATE_RANGE_END {
        STATE_RANGE_START
    } else if id >= RELATION_RANGE_START && id <= RELATION_RANGE_END {
        RELATION_RANGE_START
    } else if id >= LOGIC_RANGE_START && id <= LOGIC_RANGE_END {
        LOGIC_RANGE_START
    } else if id >= TIME_RANGE_START && id <= TIME_RANGE_END {
        TIME_RANGE_START
    } else if id >= SPACE_RANGE_START && id <= SPACE_RANGE_END {
        SPACE_RANGE_START
    } else if id >= COMMUNICATION_RANGE_START && id <= COMMUNICATION_RANGE_END {
        COMMUNICATION_RANGE_START
    } else if id >= NETWORK_RANGE_START && id <= NETWORK_RANGE_END {
        NETWORK_RANGE_START
    } else if id >= SECURITY_RANGE_START && id <= SECURITY_RANGE_END {
        SECURITY_RANGE_START
    } else if id >= COMPUTATION_RANGE_START && id <= COMPUTATION_RANGE_END {
        COMPUTATION_RANGE_START
    } else if id >= KNOWLEDGE_RANGE_START && id <= KNOWLEDGE_RANGE_END {
        KNOWLEDGE_RANGE_START
    } else if id >= VALUE_RANGE_START && id <= VALUE_RANGE_END {
        VALUE_RANGE_START
    } else if id >= DEVICE_RANGE_START && id <= DEVICE_RANGE_END {
        DEVICE_RANGE_START
    } else if id >= TOGARA_RANGE_START && id <= TOGARA_RANGE_END {
        TOGARA_RANGE_START
    } else {
        0
    }
}
