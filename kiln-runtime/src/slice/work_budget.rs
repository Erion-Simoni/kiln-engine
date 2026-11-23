//! Shared, bounded work across root/lifecycle/UI/attachment invocations.
use super::SliceSession;
use kiln_ir::{HostContext, Program, Vm, VmError};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

#[derive(Clone)]
pub(super) struct ScriptWorkBudget {
    limit: u64,
    remaining: Arc<AtomicU64>,
}
impl ScriptWorkBudget {
    pub(super) fn new(limit: u64) -> Self {
        Self {
            limit,
            remaining: Arc::new(AtomicU64::new(limit)),
        }
    }
    pub(super) fn reset(&self) {
        self.remaining.store(self.limit, Ordering::Relaxed);
    }
    pub(super) fn execute<H: HostContext>(
        &self,
        vm: &mut Vm<H>,
        program: &Program,
    ) -> Result<(), VmError> {
        // Reserve once per invocation, never once per opcode. Unused allowance
        // is returned on success and failure. Attached VMs share this pool.
        let allowance = self.remaining.swap(0, Ordering::Relaxed);
        let (result, used) = vm.execute_with_allowance(program, allowance);
        self.remaining
            .fetch_add(allowance - used, Ordering::Relaxed);
        match result {
            Err(VmError::InstructionBudgetExceeded) if self.remaining.load(Ordering::Relaxed) == 0 => {
                Err(VmError::HostCallError(format!("Instruction budget exceeded: shared gameplay tick limit {} (scene, entity and event callbacks)", self.limit)))
            }
            result => result,
        }
    }
}
impl SliceSession {
    /// Consumed and configured total instructions for the latest tick/cleanup.
    pub fn script_instruction_usage(&self) -> (u64, u64) {
        let remaining = self.work_budget.remaining.load(Ordering::Relaxed);
        (
            self.work_budget.limit.saturating_sub(remaining),
            self.work_budget.limit,
        )
    }
}
