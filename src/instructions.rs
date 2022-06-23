use crate::registers::Register;
use unescape::unescape;

#[derive(PartialEq, Debug)]
pub enum Instruction {
    MOVreg {
        destination: Register,
        source: Register,
    },
    MOVimm {
        destination: Register,
        operand: i64,
    },

    PUSHreg {
        source: Register,
    },
    POPreg {
        destination: Register,
    },

    // The gnu assembler seems to treat `push 1` as pushing a 64-bit value
    PUSHimm {
        source: i64,
    },

    ADDimm {
        destination: Register,
        operand: i64,
    },
    ADDreg {
        destination: Register,
        source: Register,
    },

    SUBimm {
        destination: Register,
        operand: i64,
    },
    SUBreg {
        destination: Register,
        source: Register,
    },

    ANDimm {
        destination: Register,
        operand: i64,
    },
    ANDreg {
        destination: Register,
        source: Register,
    },

    CMPreg {
        src1: Register,
        src2: Register,
    },
    CMPimm {
        src1: Register,
        operand: i64,
    },

    NOTreg {
        destination: Register,
    },

    TESTreg {
        src1: Register,
        src2: Register,
    },
    TESTimm {
        src1: Register,
        operand: i64,
    },

    JMPlabel {
        target: JumpTarget,
        condition: JumpCondition,
    },

    CALLlabel {
        label: String,
    },
    RET {},

    SYSCALL {},
    // TODO: call label, ret, jmp, jl jg jz, syscall, lea
}

#[derive(PartialEq, Debug)]
pub enum JumpTarget {
    Relative { forwards: bool, label: String },
    Absolute { label: String },
}

#[derive(PartialEq, Debug)]
pub enum JumpCondition {
    None,
    Zero,
    Less,
    Greater,
}

fn parse_instruction(line: &str) -> Result<Instruction, String> {
    // Split e.g. `mov rax, 0` into `"mov", "rax", "0"`
    let split: Vec<&str> = line
        .split([' ', ',', '\t'])
        .filter(|s| !s.is_empty())
        .collect();

    let mnem = split[0].to_uppercase();

    match (mnem.as_str(), split.len() - 1) {
        ("MOV", 2) => Ok(Instruction::MOVimm {
            destination: Register::parse(split[1].to_string())?,
            operand: parse_immediate_arg(split[2])?,
        }),
        ("PUSH", 1) => match parse_1_instruction_arg(split)? {
            RegOrImmediate::Register { r } => {
                if r.size == 1 {
                    Err(format!("PUSH not supported for register {} of size 1", r.name).to_string())
                } else {
                    Ok(Instruction::PUSHreg { source: r })
                }
            }
            // If it's not a register, it could be an immediate value
            RegOrImmediate::Immediate { i } => Ok(Instruction::PUSHimm { source: i }),
        },
        ("POP", 1) => {
            let dest = Register::parse(split[1].to_string())?;
            if dest.size == 1 {
                Err(format!("POP not supported for register {} of size 1", dest.name).to_string())
            } else {
                Ok(Instruction::POPreg { destination: dest })
            }
        }
        ("NOT", 1) => Ok(Instruction::NOTreg {
            destination: Register::parse(split[1].to_string())?,
        }),
        ("JMP", 1) => Ok(Instruction::JMPlabel {
            target: parse_label(split[1].to_string())?,
            condition: JumpCondition::None,
        }),
        ("SYSCALL", 0) => Ok(Instruction::SYSCALL {}),
        ("RET", 0) => Ok(Instruction::RET {}),
        ("CALL", 1) => match Register::parse(split[1].to_string()) {
            Ok(_) => Err(format!("unspported call to register value {}", split[1])),
            Err(_) => Ok(Instruction::CALLlabel {
                label: split[1].to_string(),
            }),
        },
        ("ADD", 2) => match parse_2_instruction_args(split)? {
            (reg1, RegOrImmediate::Register { r: reg2 }) => Ok(Instruction::ADDreg {
                destination: reg1,
                source: reg2,
            }),
            (reg1, RegOrImmediate::Immediate { i }) => Ok(Instruction::ADDimm {
                destination: reg1,
                operand: i,
            }),
        },
        ("SUB", 2) => match parse_2_instruction_args(split)? {
            (reg1, RegOrImmediate::Register { r: reg2 }) => Ok(Instruction::SUBreg {
                destination: reg1,
                source: reg2,
            }),
            (reg1, RegOrImmediate::Immediate { i }) => Ok(Instruction::SUBimm {
                destination: reg1,
                operand: i,
            }),
        },
        ("AND", 2) => match parse_2_instruction_args(split)? {
            (reg1, RegOrImmediate::Register { r: reg2 }) => Ok(Instruction::ANDreg {
                destination: reg1,
                source: reg2,
            }),
            (reg1, RegOrImmediate::Immediate { i }) => Ok(Instruction::ANDimm {
                destination: reg1,
                operand: i,
            }),
        },
        ("CMP", 2) => match parse_2_instruction_args(split)? {
            (reg1, RegOrImmediate::Register { r: reg2 }) => Ok(Instruction::CMPreg {
                src1: reg1,
                src2: reg2,
            }),
            (reg1, RegOrImmediate::Immediate { i }) => Ok(Instruction::CMPimm {
                src1: reg1,
                operand: i,
            }),
        },
        ("TEST", 2) => match parse_2_instruction_args(split)? {
            (reg1, RegOrImmediate::Register { r: reg2 }) => Ok(Instruction::TESTreg {
                src1: reg1,
                src2: reg2,
            }),
            (reg1, RegOrImmediate::Immediate { i }) => Ok(Instruction::TESTimm {
                src1: reg1,
                operand: i,
            }),
        },
        _ => Err(format!("Cannot parse instruction {}", line).to_string()),
    }
}

enum RegOrImmediate {
    Register { r: Register },
    Immediate { i: i64 },
}

fn parse_label(label: String) -> Result<JumpTarget, String> {
    match Register::parse(label.to_string()) {
        Ok(_) => Err("cannot use register as label".to_string()),
        Err(_) => {
            let ends = &['b', 'f'][..];
            if label.ends_with(ends) {
                return Ok(JumpTarget::Relative {
                    forwards: label.ends_with("f"),
                    label: label[..label.len() - 1].to_string(),
                });
            } else {
                return Ok(JumpTarget::Absolute { label: label });
            }
        }
    }
}

fn parse_2_instruction_args(instruction: Vec<&str>) -> Result<(Register, RegOrImmediate), String> {
    if instruction.len() != 3 {
        panic!(
            "invalid number of operands: got {}, expected 3",
            instruction.len()
        );
    } else {
        match Register::parse(instruction[1].to_string()) {
            Ok(reg1) => match Register::parse(instruction[2].to_string()) {
                Ok(reg2) => {
                    if reg1.size == reg2.size {
                        Ok((reg1, RegOrImmediate::Register { r: reg2 }))
                    } else {
                        Err(format!("invalid use of registers {} and {} of different size in {} instruction", reg1.name, reg2.name,                         instruction[0].to_uppercase()).to_string())
                    }
                }
                _ => Ok((
                    // TODO: Check if immediate is too large for destination
                    reg1,
                    RegOrImmediate::Immediate {
                        i: parse_immediate_arg(instruction[2])?,
                    },
                )),
            },
            _ => Err(format!(
                "invalid first operand {} for {} instruction",
                instruction[1],
                instruction[0].to_uppercase()
            )),
        }
    }
}

fn parse_1_instruction_arg(instruction: Vec<&str>) -> Result<RegOrImmediate, String> {
    if instruction.len() != 2 {
        panic!(
            "invalid number of operands: got {}, expected 2",
            instruction.len()
        );
    } else {
        match Register::parse(instruction[1].to_string()) {
            Ok(reg) => Ok(RegOrImmediate::Register { r: reg }),
            _ => Ok(
                // TODO: Check if immediate is too large for destination
                RegOrImmediate::Immediate {
                    i: parse_immediate_arg(instruction[2])?,
                },
            ),
        }
    }
}

#[cfg(test)]
mod instruction_parse_test {
    use crate::{
        instructions::{parse_instruction, Instruction, JumpTarget},
        registers::Register,
    };

    use super::JumpCondition;

    #[test]
    fn mov_immediate() {
        assert_eq!(
            parse_instruction("mov rax, 53"),
            Ok(Instruction::MOVimm {
                destination: Register {
                    name: "RAX".to_string(),
                    size: 8
                },
                operand: 53,
            })
        );
        assert_eq!(
            parse_instruction("mov  eAx, 53"),
            Ok(Instruction::MOVimm {
                destination: Register {
                    name: "EAX".to_string(),
                    size: 4
                },
                operand: 53,
            })
        );
        assert_eq!(
            parse_instruction("mov  al, 'z'"),
            Ok(Instruction::MOVimm {
                destination: Register {
                    name: "AL".to_string(),
                    size: 1
                },
                operand: 122,
            })
        );
        assert_eq!(
            parse_instruction(r#"mov  al, '\n'"#),
            Ok(Instruction::MOVimm {
                destination: Register {
                    name: "AL".to_string(),
                    size: 1
                },
                operand: 10,
            })
        );
        assert_eq!(
            parse_instruction(r#"mov  al, '\t'"#),
            Ok(Instruction::MOVimm {
                destination: Register {
                    name: "AL".to_string(),
                    size: 1
                },
                operand: 9,
            })
        );
    }

    #[test]
    fn push_register() {
        assert_eq!(
            parse_instruction("push rbx"),
            Ok(Instruction::PUSHreg {
                source: Register {
                    name: "RBX".to_string(),
                    size: 8
                },
            })
        );
        assert_eq!(
            parse_instruction("push r8d"),
            Ok(Instruction::PUSHreg {
                source: Register {
                    name: "R8D".to_string(),
                    size: 4
                },
            })
        );
        assert_eq!(
            parse_instruction("push sp"),
            Ok(Instruction::PUSHreg {
                source: Register {
                    name: "SP".to_string(),
                    size: 2
                },
            })
        );
        assert_eq!(
            parse_instruction("push cl"),
            Err("PUSH not supported for register CL of size 1".to_string())
        );
    }

    #[test]
    fn pop_register() {
        assert_eq!(
            parse_instruction("pop rbx"),
            Ok(Instruction::POPreg {
                destination: Register {
                    name: "RBX".to_string(),
                    size: 8
                },
            })
        );
        assert_eq!(
            parse_instruction("pop r8d"),
            Ok(Instruction::POPreg {
                destination: Register {
                    name: "R8D".to_string(),
                    size: 4
                },
            })
        );
        assert_eq!(
            parse_instruction("pop sp"),
            Ok(Instruction::POPreg {
                destination: Register {
                    name: "SP".to_string(),
                    size: 2
                },
            })
        );
        assert_eq!(
            parse_instruction("pop cl"),
            Err("POP not supported for register CL of size 1".to_string())
        );
    }

    #[test]
    fn add_immediate() {
        assert_eq!(
            parse_instruction("add rax, 53"),
            Ok(Instruction::ADDimm {
                destination: Register {
                    name: "RAX".to_string(),
                    size: 8
                },
                operand: 53,
            })
        );
        assert_eq!(
            parse_instruction("add  eAx, 53"),
            Ok(Instruction::ADDimm {
                destination: Register {
                    name: "EAX".to_string(),
                    size: 4
                },
                operand: 53,
            })
        );
        assert_eq!(
            parse_instruction("AdD  al, 'z'"),
            Ok(Instruction::ADDimm {
                destination: Register {
                    name: "AL".to_string(),
                    size: 1
                },
                operand: 122,
            })
        );
        assert_eq!(
            parse_instruction(r#"add  al, '\n'"#),
            Ok(Instruction::ADDimm {
                destination: Register {
                    name: "AL".to_string(),
                    size: 1
                },
                operand: 10,
            })
        );
        assert_eq!(
            parse_instruction(r#"add  al, '\t'"#),
            Ok(Instruction::ADDimm {
                destination: Register {
                    name: "AL".to_string(),
                    size: 1
                },
                operand: 9,
            })
        );
    }

    #[test]
    fn not_register() {
        assert_eq!(
            parse_instruction("not rbx"),
            Ok(Instruction::NOTreg {
                destination: Register {
                    name: "RBX".to_string(),
                    size: 8
                },
            })
        );
    }

    #[test]
    fn test_immediate() {
        assert_eq!(
            parse_instruction("test rsp, 0xF"),
            Ok(Instruction::TESTimm {
                src1: Register {
                    name: "RSP".to_string(),
                    size: 8
                },
                operand: 0xf
            })
        )
    }

    #[test]
    fn sub() {
        assert_eq!(
            parse_instruction("sub rsp, 8"),
            Ok(Instruction::SUBimm {
                destination: Register {
                    name: "RSP".to_string(),
                    size: 8
                },
                operand: 8
            })
        );
        assert_eq!(
            parse_instruction("sub rsp, rbx"),
            Ok(Instruction::SUBreg {
                destination: Register {
                    name: "RSP".to_string(),
                    size: 8
                },
                source: Register {
                    name: "RBX".to_string(),
                    size: 8
                },
            })
        );
        assert_eq!(
            parse_instruction("sub rsp, ebx"),
            Err(
                "invalid use of registers RSP and EBX of different size in SUB instruction"
                    .to_string()
            )
        );
    }

    #[test]
    fn jump_unconditional() {
        assert_eq!(
            parse_instruction("jmp .LeaxWins_0"),
            Ok(Instruction::JMPlabel {
                target: JumpTarget::Absolute {
                    label: ".LeaxWins_0".to_string()
                },
                condition: JumpCondition::None
            }),
        );
        assert_eq!(
            parse_instruction("jmp 2f"),
            Ok(Instruction::JMPlabel {
                target: JumpTarget::Relative {
                    forwards: true,
                    label: "2".to_string()
                },
                condition: JumpCondition::None
            }),
        );
        assert_eq!(
            parse_instruction("jmp 1b"),
            Ok(Instruction::JMPlabel {
                target: JumpTarget::Relative {
                    forwards: false,
                    label: "1".to_string()
                },
                condition: JumpCondition::None
            }),
        )
    }

    fn jump_conditional() {
        assert_eq!(
            parse_instruction("jz 2f"),
            Ok(Instruction::JMPlabel {
                target: JumpTarget::Relative {
                    forwards: true,
                    label: "2".to_string()
                },
                condition: JumpCondition::Zero
            }),
        );
        assert_eq!(
            parse_instruction("jg label"),
            Ok(Instruction::JMPlabel {
                target: JumpTarget::Absolute {
                    label: "label".to_string()
                },
                condition: JumpCondition::Greater,
            }),
        );
        assert_eq!(
            parse_instruction("jl label"),
            Ok(Instruction::JMPlabel {
                target: JumpTarget::Absolute {
                    label: "label".to_string()
                },
                condition: JumpCondition::Less,
            }),
        );
    }

    #[test]
    fn call_label() {
        assert_eq!(
            parse_instruction("call writechar"),
            Ok(Instruction::CALLlabel {
                label: "writechar".to_string(),
            }),
        );
    }

    #[test]
    fn simple_instructions() {
        assert_eq!(parse_instruction("Syscall"), Ok(Instruction::SYSCALL {}),);
        assert_eq!(parse_instruction("rEt"), Ok(Instruction::RET {}),);
    }
}

fn parse_immediate_arg(nstr: &str) -> Result<i64, String> {
    match i64::from_str_radix(nstr, 10) {
        Ok(i) => Ok(i),
        _ => {
            if nstr.starts_with("0x") {
                match i64::from_str_radix(&nstr[2..], 16) {
                    Ok(i) => Ok(i),
                    _ => Err(format!("invalid integer constant {}", nstr).to_string()),
                }
            } else {
                parse_as_char_constant(nstr)
            }
        }
    }
}

fn parse_as_char_constant(nstr: &str) -> Result<i64, String> {
    match unescape(nstr) {
        Some(s) => {
            let chars: Vec<char> = s.chars().collect();
            match s.len() {
                1 => Ok(chars[0] as i64),
                3 => {
                    if chars[0] == '\'' && chars[0] == chars[2] {
                        Ok(chars[1] as i64)
                    } else {
                        Err(format!("invalid quotes in string {}, expected \"'\"", nstr))
                    }
                }
                _ => Err(format!(
                    "invalid string length {} for {} (from {})",
                    s.len(),
                    s,
                    nstr
                )),
            }
        }
        None => Err(format!("char unescape not possible for {nstr}")),
    }
}