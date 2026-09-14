//! Capability-Based Security System
//! 
//! Priority 5: Permissions Database
//! Classification: RUNTIME
//! 
//! Capability tokens for fine-grained access control.

use spin::Mutex;

/// Capability ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapabilityId(pub u128);

/// Principal ID (user or process)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrincipalId(pub u64);

/// Resource ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceId(pub u64);

/// Capability scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityScope {
    /// Read-only access
    Read,
    /// Write-only access
    Write,
    /// Read and write access
    ReadWrite,
    /// Full control (including delete)
    Full,
}

/// Capability token
pub struct Capability {
    pub id: CapabilityId,
    pub principal: PrincipalId,
    pub resource: ResourceId,
    pub scope: CapabilityScope,
    pub expires_at: u64, // Timestamp
    pub generation: u64, // Anti-replay
}

impl Capability {
    /// Create new capability
    pub fn new(
        id: CapabilityId,
        principal: PrincipalId,
        resource: ResourceId,
        scope: CapabilityScope,
        expires_at: u64,
        generation: u64,
    ) -> Self {
        Self {
            id,
            principal,
            resource,
            scope,
            expires_at,
            generation,
        }
    }

    /// Check if capability is valid
    pub fn is_valid(&self, current_time: u64) -> bool {
        current_time < self.expires_at
    }

    /// Check if capability allows operation
    pub fn allows(&self, operation: Operation) -> bool {
        match (self.scope, operation) {
            (CapabilityScope::Read, Operation::Read) => true,
            (CapabilityScope::Write, Operation::Write) => true,
            (CapabilityScope::ReadWrite, Operation::Read) => true,
            (CapabilityScope::ReadWrite, Operation::Write) => true,
            (CapabilityScope::Full, _) => true,
            _ => false,
        }
    }
}

/// Operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Read,
    Write,
    Execute,
    Delete,
}

/// Access Control List entry
pub struct AclEntry {
    pub principal: PrincipalId,
    pub resource: ResourceId,
    pub permissions: Permissions,
}

/// Permissions bitmask
#[derive(Debug, Clone, Copy)]
pub struct Permissions(u32);

impl Permissions {
    pub const NONE: Self = Self(0);
    pub const READ: Self = Self(1 << 0);
    pub const WRITE: Self = Self(1 << 1);
    pub const EXECUTE: Self = Self(1 << 2);
    pub const DELETE: Self = Self(1 << 3);
    pub const ALL: Self = Self(0xF);

    pub const fn new(bits: u32) -> Self {
        Self(bits)
    }

    pub const fn has(&self, permission: Self) -> bool {
        (self.0 & permission.0) != 0
    }

    pub fn add(&mut self, permission: Self) {
        self.0 |= permission.0;
    }

    pub fn remove(&mut self, permission: Self) {
        self.0 &= !permission.0;
    }
}

/// Policy rule
pub struct PolicyRule {
    pub id: u64,
    pub principal: PrincipalId,
    pub resource: ResourceId,
    pub action: PolicyAction,
    pub effect: PolicyEffect,
}

/// Policy actions
#[derive(Debug, Clone, Copy)]
pub enum PolicyAction {
    Read,
    Write,
    Execute,
    Delete,
    Any,
}

/// Policy effects
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyEffect {
    Allow,
    Deny,
}

/// Permissions database
pub struct PermissionsDatabase {
    capabilities: Mutex<[Option<Capability>; 1024]>,
    acl: Mutex<[Option<AclEntry>; 512]>,
    policies: Mutex<[Option<PolicyRule>; 256]>,
    next_capability_id: u128,
    next_policy_id: u64,
}

impl PermissionsDatabase {
    /// Create new permissions database
    pub const fn new() -> Self {
        Self {
            capabilities: Mutex::new([None; 1024]),
            acl: Mutex::new([None; 512]),
            policies: Mutex::new([None; 256]),
            next_capability_id: 1,
            next_policy_id: 1,
        }
    }

    /// Issue a new capability
    pub fn issue_capability(
        &self,
        principal: PrincipalId,
        resource: ResourceId,
        scope: CapabilityScope,
        expires_at: u64,
    ) -> Option<CapabilityId> {
        let mut caps = self.capabilities.lock();
        
        // Find free slot
        for i in 0..1024 {
            if caps[i].is_none() {
                let cap_id = CapabilityId(self.next_capability_id);
                self.next_capability_id += 1;
                
                caps[i] = Some(Capability::new(
                    cap_id,
                    principal,
                    resource,
                    scope,
                    expires_at,
                    0, // generation
                ));
                
                return Some(cap_id);
            }
        }
        
        None // No free slots
    }

    /// Revoke a capability
    pub fn revoke_capability(&self, cap_id: CapabilityId) -> bool {
        let mut caps = self.capabilities.lock();
        
        for i in 0..1024 {
            if let Some(ref cap) = caps[i] {
                if cap.id == cap_id {
                    caps[i] = None;
                    return true;
                }
            }
        }
        
        false
    }

    /// Check if principal has permission
    pub fn check_permission(
        &self,
        principal: PrincipalId,
        resource: ResourceId,
        operation: Operation,
        current_time: u64,
    ) -> bool {
        let caps = self.capabilities.lock();
        
        // Check capabilities
        for cap_opt in caps.iter() {
            if let Some(cap) = cap_opt {
                if cap.principal == principal
                    && cap.resource == resource
                    && cap.is_valid(current_time)
                    && cap.allows(operation)
                {
                    return true;
                }
            }
        }
        
        // Check ACL
        let acl = self.acl.lock();
        for entry_opt in acl.iter() {
            if let Some(entry) = entry_opt {
                if entry.principal == principal && entry.resource == resource {
                    match operation {
                        Operation::Read => {
                            if entry.permissions.has(Permissions::READ) {
                                return true;
                            }
                        }
                        Operation::Write => {
                            if entry.permissions.has(Permissions::WRITE) {
                                return true;
                            }
                        }
                        Operation::Execute => {
                            if entry.permissions.has(Permissions::EXECUTE) {
                                return true;
                            }
                        }
                        Operation::Delete => {
                            if entry.permissions.has(Permissions::DELETE) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        
        false
    }

    /// Add ACL entry
    pub fn add_acl_entry(
        &self,
        principal: PrincipalId,
        resource: ResourceId,
        permissions: Permissions,
    ) -> bool {
        let mut acl = self.acl.lock();
        
        for entry_opt in acl.iter() {
            if entry_opt.is_none() {
                *entry_opt = Some(AclEntry {
                    principal,
                    resource,
                    permissions,
                });
                return true;
            }
        }
        
        false
    }

    /// Add policy rule
    pub fn add_policy_rule(
        &self,
        principal: PrincipalId,
        resource: ResourceId,
        action: PolicyAction,
        effect: PolicyEffect,
    ) -> Option<u64> {
        let mut policies = self.policies.lock();
        
        for policy_opt in policies.iter() {
            if policy_opt.is_none() {
                let policy_id = self.next_policy_id;
                self.next_policy_id += 1;
                
                *policy_opt = Some(PolicyRule {
                    id: policy_id,
                    principal,
                    resource,
                    action,
                    effect,
                });
                
                return Some(policy_id);
            }
        }
        
        None
    }
}

impl Default for PermissionsDatabase {
    fn default() -> Self {
        Self::new()
    }
}

/// Global permissions database
pub static PERMISSIONS_DB: Mutex<Option<PermissionsDatabase>> = Mutex::new(None);

/// Initialize permissions database
pub fn init_permissions_db() {
    let mut db_opt = PERMISSIONS_DB.lock();
    *db_opt = Some(PermissionsDatabase::new());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permissions() {
        let mut perms = Permissions::NONE;
        assert!(!perms.has(Permissions::READ));
        
        perms.add(Permissions::READ);
        assert!(perms.has(Permissions::READ));
        
        perms.remove(Permissions::READ);
        assert!(!perms.has(Permissions::READ));
    }

    #[test]
    fn test_capability_validation() {
        let cap = Capability::new(
            CapabilityId(1),
            PrincipalId(100),
            ResourceId(200),
            CapabilityScope::Read,
            1000, // expires_at
            0,    // generation
        );
        
        assert!(cap.is_valid(500));
        assert!(!cap.is_valid(1500));
        assert!(cap.allows(Operation::Read));
        assert!(!cap.allows(Operation::Write));
    }
}
