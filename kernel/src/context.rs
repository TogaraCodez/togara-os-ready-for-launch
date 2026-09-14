//! Context Switching Integration
//! 
//! Part 10 of TOGARA OS Platform Roadmap
//! Classification: RUNTIME
//! 
//! Integrates assembly context switch with the scheduler.

use crate::scheduler::{CpuContext, Task, TaskState, Scheduler};

extern "C" {
    /// Assembly context switch function
    fn switch_context(current: *mut CpuContext, next: *const CpuContext);
}

/// Perform a context switch between tasks
/// 
/// # Safety
/// - Both tasks must have valid stack pointers
/// - This function does not return in the traditional sense
pub unsafe fn context_switch(current_task: &mut Task, next_task: &mut Task) {
    // Save current task state
    current_task.state = TaskState::Ready;
    
    // Set next task as running
    next_task.state = TaskState::Running;
    
    // Perform the actual context switch
    switch_context(&mut current_task.context, &next_task.context);
}

/// Create an initial idle task
pub fn create_idle_task() -> Task {
    use crate::scheduler::TaskId;
    
    Task::new(TaskId(0), idle_entry as usize)
}

/// Idle task entry point
fn idle_entry() -> ! {
    loop {
        core::hint::spin_loop();
    }
}

/// Create an initial userspace task
/// 
/// # Safety
/// - Entry point must be a valid userspace address
/// - Stack must be properly allocated
pub unsafe fn create_userspace_task(id: u64, entry: u64, stack_top: u64) -> Task {
    use crate::scheduler::TaskId;
    use crate::gdt::Gdt;
    
    let mut task = Task::new(TaskId(id), entry as usize);
    
    // Set up userspace stack
    task.context.rsp = stack_top;
    
    // Set up segment selectors for Ring 3
    if let Some(gdt) = &crate::gdt::GDT {
        let selectors = gdt.selectors();
        task.context.cs = selectors.user_code as u64;
        task.context.ss = selectors.user_data as u64;
    }
    
    // Set flags for userspace (interrupts enabled)
    task.context.rflags = 0x202;
    
    task
}

/// Initialize and start the scheduler
/// 
/// # Safety
/// - Must be called after GDT, IDT, and timer are initialized
pub unsafe fn start_scheduler(scheduler: &mut Scheduler) -> ! {
    // Create idle task
    let idle = create_idle_task();
    scheduler.add_task(idle).unwrap();
    
    // Set idle as current
    scheduler.set_running(crate::scheduler::TaskId(0));
    
    // Enter scheduler loop
    scheduler_loop(scheduler)
}

/// Main scheduler loop
/// 
/// # Safety
/// - Requires timer interrupts to be enabled
fn scheduler_loop(scheduler: &mut Scheduler) -> ! {
    loop {
        // Get next runnable task
        if let Some(next) = scheduler.get_next_task() {
            let next_id = next.id;
            
            // Get current task
            if let Some(current_id) = scheduler.current_task_id() {
                if current_id != next_id {
                    // Get mutable references to both tasks
                    if let (Some(current), Some(next)) = (
                        scheduler.get_task_mut(current_id),
                        scheduler.get_task_mut(next_id),
                    ) {
                        // Perform context switch
                        unsafe {
                            context_switch(current, next);
                        }
                    }
                }
            } else {
                // No current task, just start the next one
                scheduler.set_running(next_id);
            }
        }
        
        // Wait for next timer interrupt
        core::hint::spin_loop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idle_task_creation() {
        let task = create_idle_task();
        assert_eq!(task.id, crate::scheduler::TaskId(0));
        assert_eq!(task.state, TaskState::Ready);
    }
}
