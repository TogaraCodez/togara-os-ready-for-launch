//! Preemptive Task Scheduler
//! 
//! Part 9-10 of TOGARA OS Platform Roadmap
//! Classification: RUNTIME
//! 
//! Implements a simple preemptive scheduler.

/// Task state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Sleeping,
    Blocked,
    Exited,
}

/// Task ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub u64);

/// CPU context saved during context switch
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CpuContext {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rbp: u64,
    pub rbx: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rax: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

impl Default for CpuContext {
    fn default() -> Self {
        Self {
            r15: 0, r14: 0, r13: 0, r12: 0, r11: 0, r10: 0, r9: 0, r8: 0,
            rdi: 0, rsi: 0, rbp: 0, rbx: 0, rdx: 0, rcx: 0, rax: 0,
            rip: 0, cs: 0, rflags: 0, rsp: 0, ss: 0,
        }
    }
}

/// Task control block
pub struct Task {
    pub id: TaskId,
    pub state: TaskState,
    pub priority: u8,
    pub vruntime: u64,
    pub context: CpuContext,
    pub stack: Option<Box<[u8; 8192]>>,  // 8KB stack
}

impl Task {
    /// Create a new task
    pub fn new(id: TaskId, entry: usize) -> Self {
        let stack = Some(Box::new([0u8; 8192]));
        let stack_top = stack.as_ref().map(|s| s.as_ptr() as u64 + s.len() as u64).unwrap_or(0);
        
        Self {
            id,
            state: TaskState::Ready,
            priority: 10,
            vruntime: 0,
            context: CpuContext {
                rip: entry as u64,
                rsp: stack_top,
                rflags: 0x202,  // Interrupts enabled
                ..Default::default()
            },
            stack,
        }
    }
}

/// Scheduler
pub struct Scheduler {
    tasks: [Option<Task>; 256],
    current_task: Option<TaskId>,
    next_id: u64,
}

impl Scheduler {
    /// Create a new scheduler
    pub const fn new() -> Self {
        const NONE: Option<Task> = None;
        Self {
            tasks: [NONE; 256],
            current_task: None,
            next_id: 1,
        }
    }

    /// Add a task to the scheduler
    pub fn add_task(&mut self, task: Task) -> Result<TaskId, SchedulerError> {
        for i in 0..256 {
            if self.tasks[i].is_none() {
                let id = task.id;
                self.tasks[i] = Some(task);
                return Ok(id);
            }
        }
        Err(SchedulerError::TaskLimitReached)
    }

    /// Get the next runnable task
    pub fn get_next_task(&mut self) -> Option<&mut Task> {
        for task_opt in &mut self.tasks {
            if let Some(task) = task_opt {
                if task.state == TaskState::Ready {
                    return Some(task);
                }
            }
        }
        None
    }

    /// Set current task as running
    pub fn set_running(&mut self, id: TaskId) {
        self.current_task = Some(id);
        if let Some(task) = self.get_task_mut(id) {
            task.state = TaskState::Running;
        }
    }

    /// Get task by ID
    pub fn get_task(&self, id: TaskId) -> Option<&Task> {
        self.tasks.iter().flatten().find(|t| t.id == id)
    }

    /// Get mutable task by ID
    pub fn get_task_mut(&mut self, id: TaskId) -> Option<&mut Task> {
        self.tasks.iter_mut().flatten().find(|t| t.id == id)
    }

    /// Get current task ID
    pub const fn current_task_id(&self) -> Option<TaskId> {
        self.current_task
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Scheduler error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerError {
    TaskLimitReached,
    TaskNotFound,
    InvalidState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new(TaskId(1), 0x1000);
        assert_eq!(task.id, TaskId(1));
        assert_eq!(task.state, TaskState::Ready);
        assert!(task.stack.is_some());
    }

    #[test]
    fn test_scheduler_add_task() {
        let mut scheduler = Scheduler::new();
        let task = Task::new(TaskId(1), 0x1000);
        
        assert!(scheduler.add_task(task).is_ok());
        assert!(scheduler.get_task(TaskId(1)).is_some());
    }

    #[test]
    fn test_scheduler_get_next() {
        let mut scheduler = Scheduler::new();
        let task = Task::new(TaskId(1), 0x1000);
        scheduler.add_task(task).unwrap();
        
        let next = scheduler.get_next_task();
        assert!(next.is_some());
        assert_eq!(next.unwrap().id, TaskId(1));
    }
}
