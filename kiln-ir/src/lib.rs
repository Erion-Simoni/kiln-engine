//! Kiln IR — bytecode VM with host call interface.

pub mod debugger;
pub mod opcode;
pub mod program;
pub mod value;
pub mod vm;

pub use debugger::{Breakpoint, DebugEvent, Debugger, DebugState, InstructionLocation, StepMode, WatchExpression};
pub use opcode::Opcode;
pub use program::{Instruction, Label, Program, ProgramError};
pub use value::Value;
pub use vm::{HostContext, Vm, VmError};
