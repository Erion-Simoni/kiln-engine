//! Kiln IR — bytecode VM with host call interface.

pub mod opcode;
pub mod value;
pub mod program;
pub mod vm;

pub use opcode::Opcode;
pub use value::Value;
pub use program::{Program, Label};
pub use vm::{Vm, HostContext};
