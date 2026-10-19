//! Kiln IR — bytecode VM with host call interface.

pub mod opcode;
pub mod program;
pub mod value;
pub mod vm;

pub use opcode::Opcode;
pub use program::{Instruction, Label, Program};
pub use value::Value;
pub use vm::{HostContext, Vm, VmError};
