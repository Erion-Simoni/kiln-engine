//! Bytecode opcodes for the Kiln VM.

/// Bytecode opcodes for the Kiln VM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {
    // Stack operations
    PushConst = 0,
    Pop,
    Dup,
    Swap,

    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Neg,

    // Comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,

    // Control flow
    Jump,
    JumpIf,
    JumpIfNot,
    Call,
    Return,

    // Host calls
    HostCall,
}

impl Opcode {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Opcode::PushConst),
            1 => Some(Opcode::Pop),
            2 => Some(Opcode::Dup),
            3 => Some(Opcode::Swap),
            4 => Some(Opcode::Add),
            5 => Some(Opcode::Sub),
            6 => Some(Opcode::Mul),
            7 => Some(Opcode::Div),
            8 => Some(Opcode::Neg),
            9 => Some(Opcode::Eq),
            10 => Some(Opcode::Ne),
            11 => Some(Opcode::Lt),
            12 => Some(Opcode::Le),
            13 => Some(Opcode::Gt),
            14 => Some(Opcode::Ge),
            15 => Some(Opcode::Jump),
            16 => Some(Opcode::JumpIf),
            17 => Some(Opcode::JumpIfNot),
            18 => Some(Opcode::Call),
            19 => Some(Opcode::Return),
            20 => Some(Opcode::HostCall),
            _ => None,
        }
    }
}
