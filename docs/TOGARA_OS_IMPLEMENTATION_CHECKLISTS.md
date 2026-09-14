# TOGARA OS Implementation Checklists

**Milestone-by-milestone execution plan**

*Version 1.0*

---

## How to Use

1. **One milestone at a time** - Do not skip ahead
2. **Mark each item:** `[ ]` → `[/]` → `[x]`
3. **Run baseline checks** before each milestone:
   ```bash
   cargo fmt --all && cargo check --workspace && cargo test --workspace
   ```
4. **Document risks** in `MILESTONE_NOTES.md`

---

## Milestone Overview

| # | Focus | Parts | Classification | Effort |
|---|-------|-------|----------------|--------|
| 1 | Repository baseline & boot | 0-10 | RUNTIME | 1-2 weeks |
| 2 | Processes, syscalls, IPC | 11-20 | RUNTIME | 2-3 weeks |
| 3 | Networking stack | 21-26 | INFRA | 2-3 weeks |
| 4 | Security & userspace | 27-32 | RUNTIME/INFRA | 2-3 weeks |
| 5 | **GNOSIS & TUO Core** | 33-39 | **CORE** | 3-4 weeks |
| 6 | AI runtime | 40-45 | DEFERRED/CORE | 2-3 weeks or skip |
| 7 | Desktop, TRIC, SETH | 46-50 | PRODUCT/CORE | 3-4 weeks |
| 8 | Deferred concepts | 51-54 | DEFERRED | Skip |
| 9 | Data quality, requirements | 55-60 | CORE/RUNTIME | 2-3 weeks |
| 10 | Testing & verification | 61-65 | CORE/RUNTIME | 2-3 weeks |
| 11 | Integration & QEMU | 66-69 | RUNTIME | 2-3 weeks |
| 12 | Performance & release | 70-76 | RUNTIME/PRODUCT | 2-3 weeks |
| 13 | Final gates & release | 77-80 | RUNTIME/PRODUCT | 1-2 weeks |

**Total:** 25-35 weeks (6-9 months) full, or 18-24 weeks (4-6 months) if Milestones 6 & 8 deferred

---

## Milestone 1 — Repository Baseline & Boot (Parts 0-10)

### Part 0 — Repository Baseline
- [ ] Working directory: `~/Downloads/togara-os-rebound`
- [ ] Branch: `git switch -c feat/trinity-general-purpose-os`
- [ ] `cargo fmt --all` passes
- [ ] `cargo check --workspace` passes
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes

### Part 1 — Architecture Directory
- [ ] Create `kernel/src/arch/x86_64` through `kernel/src/desktop`
- [ ] Create `userspace/init` through `userspace/desktop`
- [ ] Create `tests/integration` through `tests/fuzz`
- [ ] Create `tools`

### Part 2 — Boot System
- [ ] `BootContext` struct with memory_map, framebuffer, physical_memory_offset
- [ ] `MemoryRegion` struct with start, len, usable
- [ ] Integer safety: `checked_add()` for all `start + length`
- [ ] BIOS boot prints: `TOGARA OS TRINITY`, `BOOT OK`, `MEMORY MAP OK`, `FRAMEBUFFER OK`
- [ ] UEFI boot prints same messages

### Part 3 — Physical Memory Manager
- [ ] Page size: 4096
- [ ] `PhysFrame` struct with number: u64
- [ ] `FrameAllocator` trait with alloc/dealloc
- [ ] Bitmap allocator implemented
- [ ] Invariants: Allocated ∩ Free = ∅, Reserved ∩ Free = ∅
- [ ] Tests: 10,000 allocations unique, 100,000 randomized ops

### Part 4 — Virtual Memory
- [ ] x86_64 page table structures (PML4, PDPT, PD, PT)
- [ ] `PagePerm` struct with writable, executable, user
- [ ] W^X invariant: W ∧ X = false
- [ ] Kernel mappings: text (RX), readonly (R/NX), data (RW/NX)
- [ ] User/kernel isolation test passes

### Part 5 — Kernel Heap
- [ ] Global allocator configured
- [ ] Proven allocator integrated
- [ ] Alignment tests: 1, 8, 16, 64, 4096 bytes
- [ ] OOM returns explicit failure

### Part 6 — GDT / TSS / IDT
- [ ] GDT entries: Kernel Code/Data, User Code/Data, TSS
- [ ] TSS with dedicated double-fault stack (guard page)
- [ ] IDT for #PF, #GP, #DF, #UD (minimum)

### Part 7 — Page Fault Engine
- [ ] `PageFaultInfo` struct
- [ ] User fault → terminate process
- [ ] Kernel fault → diagnostic panic/halt

### Part 8 — Timer
- [ ] Timer frequency configured (e.g., 100 Hz)
- [ ] Tick calculation: tick = 1/f

### Part 9 — Preemptive Scheduler
- [ ] `TaskState` enum
- [ ] `Task` struct with id, state, priority, vruntime
- [ ] Timeslice calculation
- [ ] Scheduler algorithm implemented

### Part 10 — Context Switching
- [ ] Assembly context switch (push/pop rbx, rbp, r12-r15)
- [ ] Tested before userspace

### Milestone 1 Acceptance
- [ ] System boots (BIOS & UEFI) with OK messages
- [ ] PMM allocates/frees 100k frames
- [ ] VMM enforces user/kernel isolation
- [ ] All cargo checks pass
- [ ] QEMU boot test passes

---

## Milestone 5 — GNOSIS & TUO Core (Parts 33-39) ⭐

### Part 34 — GNOSIS
- [ ] `CanonicalState` type defined
- [ ] GNOSIS as authoritative state repository
- [ ] Invariant: DerivedState ↛ CanonicalState

### Part 35 — TUO
- [ ] `EpistemicStatus` enum: Unknown, Proposed, Observed, Corroborated, Verified, Rejected
- [ ] `Claim` struct with subject, predicate, value, status, provenance, authority
- [ ] Epistemic distinctions enforced

### Part 36 — Epistemic Mathematics
- [ ] Evidence support function
- [ ] Distinctions: Confidence ≠ Truth, Consensus ≠ Authority

### Part 37 — Canonical Commit
- [ ] Commit rule implemented
- [ ] Non-canonical states tracked

### Part 38 — Provenance
- [ ] `Provenance` struct with sources, transformation, model, assumptions
- [ ] Derived object tracking

### Part 39 — Deterministic Replay
- [ ] State transition: S_{t+1} = F(S_t, I_t)
- [ ] `ReplayHeader` struct
- [ ] Record → replay → verify

### Milestone 5 Acceptance
- [ ] GNOSIS maintains canonical state
- [ ] TUO enforces epistemic distinctions
- [ ] Provenance tracked
- [ ] Deterministic replay works
- [ ] All tests pass

---

## Milestone 13 — Final Gates & Release (Parts 77-80)

### Part 77 — Coding-LLM Protocol
- [ ] 18-step process followed for each milestone
- [ ] Output format documented

### Part 78 — Code Review
- [ ] 20-item review checklist used
- [ ] No critical findings

### Part 79 — Release Authority
- [ ] Decision: PASS / CONDITIONAL PASS / FAIL
- [ ] 14 hard blockers checked

### Part 80 — Final Release Gates
- [ ] All 80+ gates passed
- [ ] Release manifest, SBOM, signatures generated
- [ ] BIOS and UEFI images created

### Milestone 13 Acceptance
- [ ] Release authority: PASS
- [ ] All gates passed
- [ ] Product ready for market

---

*Execute one milestone at a time. Do not skip ahead.*
