use crate::long_mode::read_E;
use crate::long_mode::read_E_xmm;
use crate::long_mode::read_E_ymm;
use crate::long_mode::read_imm_unsigned;
use crate::long_mode::read_modrm;
use crate::long_mode::DecodeError;
use crate::long_mode::Instruction;
use crate::long_mode::Opcode;
use crate::long_mode::OperandSpec;
use crate::long_mode::RegSpec;
use crate::long_mode::RegisterBank;

#[derive(Debug)]
enum VEXOpcodeMap {
    Map0F,
    Map0F38,
    Map0F3A,
}

#[derive(Debug)]
enum VEXOpcodePrefix {
    None,
    Prefix66,
    PrefixF3,
    PrefixF2,
}

#[allow(non_camel_case_types)]
#[derive(Debug)]
enum VEXOperandCode {
    Nothing,
    VPS_71,
    VPS_71_L,
    VPS_72,
    VPS_72_L,
    VPS_73,
    VPS_73_L,
    VMOVSS_10,
    VMOVSD_10,
    VMOVSD_11,
    VMOVSS_11,
    E_G_xmm,
    U_G_xmm,
    M_G_xmm,
    G_M_xmm,
    G_U_xmm,
    E_G_xmm_imm8,
    U_G_xmm_imm8,
    E_G_ymm,
    U_G_ymm,
    M_G_ymm,
    G_E_ymm,
    G_M_ymm,
    G_U_ymm,
    E_V_G_ymm,
    E_V_G_xmm,
    E_xmm_G_ymm_imm8,
    Ev_G_xmm_imm8,
    G_Ex_V_xmm,
    G_Ey_V_ymm,
    G_E_xmm,
    G_E_xmm_imm8,
    G_E_ymm_imm8,
    G_xmm_E_xmm,
    G_xmm_E_ymm,
    G_ymm_E_xmm,
    G_ymm_E_ymm,
    G_V_ymm_E_xmm,
    M_V_G_xmm,
    M_V_G_ymm,
    G_V_E_xmm,
    G_V_E_xmm_imm8,
    G_V_E_xmm_xmm4,
    G_V_E_ymm,
    G_V_E_ymm_imm8,
    G_V_E_ymm_ymm4,
    G_V_M_xmm,
    G_V_M_ymm,
    V_xmm_G_ymm_E_ymm_imm8,
    V_ymm_G_ymm_E_xmm_imm8,
    G_V_xmm_Ew_imm8,
    Eq_G_xmm,
    Ed_G_xmm,
    G_xmm_Ed,
    G_xmm_Eq,
}

#[inline(never)]
pub(crate) fn three_byte_vex<T: Iterator<Item = u8>>(
    bytes: &mut T,
    instruction: &mut Instruction,
    mut length: u8,
) -> Result<(), DecodeError> {
    let vex_byte_one = bytes.next().ok_or(DecodeError::ExhaustedInput)?;
    let vex_byte_two = bytes.next().ok_or(DecodeError::ExhaustedInput)?;
    length += 2;
    let p = vex_byte_two & 0x03;
    let p = match p {
        0x00 => VEXOpcodePrefix::None,
        0x01 => VEXOpcodePrefix::Prefix66,
        0x02 => VEXOpcodePrefix::PrefixF3,
        0x03 => VEXOpcodePrefix::PrefixF2,
        _ => {
            unreachable!("p is two bits");
        }
    };
    let m = vex_byte_one & 0b11111;
    //    println!("m: {:05b}", m);
    let m = match m {
        0b00001 => VEXOpcodeMap::Map0F,
        0b00010 => VEXOpcodeMap::Map0F38,
        0b00011 => VEXOpcodeMap::Map0F3A,
        _ => {
            instruction.opcode = Opcode::Invalid;
            return Err(DecodeError::InvalidOpcode);
        }
    };
    instruction.vex_reg = RegSpec {
        bank: RegisterBank::X,
        num: ((vex_byte_two >> 3) & 0b1111) ^ 0b1111,
    };
    instruction.prefixes.vex_from_c4(vex_byte_one, vex_byte_two);

    read_vex_instruction(m, bytes, instruction, &mut length, p)?;
    instruction.length = length;
    Ok(())
}

pub(crate) fn two_byte_vex<T: Iterator<Item = u8>>(
    bytes: &mut T,
    instruction: &mut Instruction,
    mut length: u8,
) -> Result<(), DecodeError> {
    let vex_byte = bytes.next().ok_or(DecodeError::ExhaustedInput)?;
    length += 1;
    let p = vex_byte & 0x03;
    let p = match p {
        0x00 => VEXOpcodePrefix::None,
        0x01 => VEXOpcodePrefix::Prefix66,
        0x02 => VEXOpcodePrefix::PrefixF3,
        0x03 => VEXOpcodePrefix::PrefixF2,
        _ => {
            unreachable!("p is two bits");
        }
    };
    instruction.vex_reg = RegSpec {
        bank: RegisterBank::X,
        num: ((vex_byte >> 3) & 0b1111) ^ 0b1111,
    };
    instruction.prefixes.vex_from_c5(vex_byte);

    read_vex_instruction(VEXOpcodeMap::Map0F, bytes, instruction, &mut length, p)?;
    instruction.length = length;
    Ok(())
}

fn read_vex_operands<T: Iterator<Item = u8>>(
    bytes: &mut T,
    instruction: &mut Instruction,
    length: &mut u8,
    operand_code: VEXOperandCode,
) -> Result<(), DecodeError> {
    //    println!("operand code: {:?}", operand_code);
    match operand_code {
        VEXOperandCode::VPS_71 => {
            let modrm = read_modrm(bytes, length)?;
            match (modrm >> 3) & 0b111 {
                0b010 => {
                    instruction.opcode = Opcode::VPSRLW;
                }
                0b100 => {
                    instruction.opcode = Opcode::VPSRAW;
                }
                0b110 => {
                    instruction.opcode = Opcode::VPSLLW;
                }
                _ => {
                    return Err(DecodeError::InvalidOpcode);
                }
            }
            /*
            0x71 => (Opcode::VPSLLW, if L {
                VEXOperandCode::G_E_ymm_imm8
            } else {
                VEXOperandCode::G_E_ymm_imm8
            }),
            0x71 => (Opcode::VPSRAW, if L {
                VEXOperandCode::G_E_ymm_imm8
            } else {
                VEXOperandCode::G_E_ymm_imm8
            }),
            0x71 => (Opcode::VPSRLW, if L {
                VEXOperandCode::G_V_E_ymm
            } else {
                VEXOperandCode::G_V_E_ymm
            }),
            */
            Err(DecodeError::IncompleteDecoder) // :)
        }
        VEXOperandCode::VPS_71_L => {
            let modrm = read_modrm(bytes, length)?;
            match (modrm >> 3) & 0b111 {
                0b010 => {
                    instruction.opcode = Opcode::VPSRLW;
                }
                0b100 => {
                    instruction.opcode = Opcode::VPSRAW;
                }
                0b110 => {
                    instruction.opcode = Opcode::VPSLLW;
                }
                _ => {
                    return Err(DecodeError::InvalidOpcode);
                }
            }
            /*
            0x71 => (Opcode::VPSLLW, if L {
                VEXOperandCode::G_E_ymm_imm8
            } else {
                VEXOperandCode::G_E_ymm_imm8
            }),
            0x71 => (Opcode::VPSRAW, if L {
                VEXOperandCode::G_E_ymm_imm8
            } else {
                VEXOperandCode::G_E_ymm_imm8
            }),
            0x71 => (Opcode::VPSRLW, if L {
                VEXOperandCode::G_V_E_ymm
            } else {
                VEXOperandCode::G_V_E_ymm
            }),
            */
            Err(DecodeError::IncompleteDecoder) // :)
        }
        VEXOperandCode::VPS_72 => {
            let modrm = read_modrm(bytes, length)?;
            match (modrm >> 3) & 0b111 {
                0b010 => {
                    instruction.opcode = Opcode::VPSRLD;
                }
                0b100 => {
                    instruction.opcode = Opcode::VPSRAD;
                }
                0b110 => {
                    instruction.opcode = Opcode::VPSLLD;
                }
                _ => {
                    return Err(DecodeError::InvalidOpcode);
                }
            }
            /*
            0x72 => (Opcode::VPSLLD, if L {
                VEXOperandCode::G_E_ymm_imm8
            } else {
                VEXOperandCode::G_E_ymm_imm8
            }),
            0x72 => (Opcode::VPSRAD, if L {
                VEXOperandCode::G_E_ymm_imm8
            } else {
                VEXOperandCode::G_E_ymm_imm8
            }),
            0x72 => (Opcode::VPSRLD, if L {
                VEXOperandCode::G_E_ymm_imm8
            } else {
                VEXOperandCode::G_E_ymm_imm8
            }),
            */
            Err(DecodeError::IncompleteDecoder) // :)
        }
        VEXOperandCode::VPS_72_L => {
            let modrm = read_modrm(bytes, length)?;
            match (modrm >> 3) & 0b111 {
                0b010 => {
                    instruction.opcode = Opcode::VPSRLD;
                }
                0b100 => {
                    instruction.opcode = Opcode::VPSRAD;
                }
                0b110 => {
                    instruction.opcode = Opcode::VPSLLD;
                }
                _ => {
                    return Err(DecodeError::InvalidOpcode);
                }
            }
            /*
            0x72 => (Opcode::VPSLLD, if L {
                VEXOperandCode::G_E_ymm_imm8
            } else {
                VEXOperandCode::G_E_ymm_imm8
            }),
            0x72 => (Opcode::VPSRAD, if L {
                VEXOperandCode::G_E_ymm_imm8
            } else {
                VEXOperandCode::G_E_ymm_imm8
            }),
            0x72 => (Opcode::VPSRLD, if L {
                VEXOperandCode::G_E_ymm_imm8
            } else {
                VEXOperandCode::G_E_ymm_imm8
            }),
            */
            Err(DecodeError::IncompleteDecoder) // :)
        }
        VEXOperandCode::VPS_73 => {
            let modrm = read_modrm(bytes, length)?;
            match (modrm >> 3) & 0b111 {
                0b010 => {
                    instruction.opcode = Opcode::VPSRLQ;
                }
                0b011 => {
                    instruction.opcode = Opcode::VPSRLDQ;
                }
                0b110 => {
                    instruction.opcode = Opcode::VPSLLQ;
                }
                0b111 => {
                    instruction.opcode = Opcode::VPSLLDQ;
                }
                _ => {
                    return Err(DecodeError::InvalidOpcode);
                }
            }
            // VEXOperandCode::G_E_xmm_imm8 ? this is reg1, reg2, imm8, but r is used for
            // picking the opcode. is one of these actually the vex reg?
            Err(DecodeError::IncompleteDecoder) // :)
        }
        VEXOperandCode::VPS_73_L => {
            let modrm = read_modrm(bytes, length)?;
            match (modrm >> 3) & 0b111 {
                0b000 | 0b001 | 0b100 | 0b101 => {
                    return Err(DecodeError::InvalidOpcode);
                }
                0b010 => {
                    instruction.opcode = Opcode::VPSRLQ;
                }
                0b011 => {
                    instruction.opcode = Opcode::VPSRLDQ;
                }
                0b110 => {
                    instruction.opcode = Opcode::VPSLLQ;
                }
                0b111 => {
                    instruction.opcode = Opcode::VPSLLDQ;
                }
                _ => {
                    unreachable!("r is only three bits");
                }
            }
            // VEXOperandCode::G_E_ymm_imm8 ? this is reg1, reg2, imm8, but r is used for
            // picking the opcode. is one of these actually the vex reg?
            Err(DecodeError::IncompleteDecoder) // :)
        }
        VEXOperandCode::VMOVSS_10 | VEXOperandCode::VMOVSD_10 => {
            let modrm = read_modrm(bytes, length)?;
            instruction.modrm_rrr = RegSpec::from_parts(
                (modrm >> 3) & 7,
                instruction.prefixes.vex().r(),
                RegisterBank::X,
            );
            let mem_oper = read_E_xmm(bytes, instruction, modrm, length)?;
            instruction.operands[0] = OperandSpec::RegRRR;
            match mem_oper {
                OperandSpec::RegMMM => {
                    instruction.operands[1] = OperandSpec::RegVex;
                    instruction.operands[2] = OperandSpec::RegMMM;
                }
                other => {
                    instruction.operands[1] = other;
                }
            }
            Ok(())
        }
        VEXOperandCode::VMOVSS_11 | VEXOperandCode::VMOVSD_11 => {
            let modrm = read_modrm(bytes, length)?;
            instruction.modrm_rrr = RegSpec::from_parts(
                (modrm >> 3) & 7,
                instruction.prefixes.vex().r(),
                RegisterBank::X,
            );
            let mem_oper = read_E_xmm(bytes, instruction, modrm, length)?;
            instruction.operands[1] = OperandSpec::RegRRR;
            match mem_oper {
                OperandSpec::RegMMM => {
                    instruction.operands[0] = OperandSpec::RegVex;
                    instruction.operands[2] = OperandSpec::RegMMM;
                }
                other => {
                    instruction.operands[0] = other;
                }
            }
            Ok(())
        }
        VEXOperandCode::Nothing => Ok(()),
        VEXOperandCode::Ev_G_xmm_imm8 => {
            if instruction.vex_reg.num != 0 {
                instruction.opcode = Opcode::Invalid;
                return Err(DecodeError::InvalidOperand);
            }
            let modrm = read_modrm(bytes, length)?;
            instruction.modrm_rrr = RegSpec::from_parts(
                (modrm >> 3) & 7,
                instruction.prefixes.vex().r(),
                RegisterBank::X,
            );
            let mem_oper = read_E(bytes, instruction, modrm, 8, length)?;
            instruction.operands[0] = mem_oper;
            instruction.operands[1] = OperandSpec::RegRRR;
            instruction.operands[2] = OperandSpec::ImmU8;
            instruction.imm = read_imm_unsigned(bytes, 1, length)?;
            Ok(())
        }
        _op @ VEXOperandCode::E_G_xmm
        | _op @ VEXOperandCode::U_G_xmm
        | _op @ VEXOperandCode::M_G_xmm
        | _op @ VEXOperandCode::E_G_xmm_imm8
        | _op @ VEXOperandCode::U_G_xmm_imm8 => {
            if instruction.vex_reg.num != 0 {
                instruction.opcode = Opcode::Invalid;
                return Err(DecodeError::InvalidOperand);
            }
            let modrm = read_modrm(bytes, length)?;
            instruction.modrm_rrr = RegSpec::from_parts(
                (modrm >> 3) & 7,
                instruction.prefixes.vex().r(),
                RegisterBank::X,
            );
            let mem_oper = read_E_xmm(bytes, instruction, modrm, length)?;
            instruction.operands[0] = mem_oper;
            instruction.operands[1] = OperandSpec::RegRRR;
            Ok(())
        }

        _op @ VEXOperandCode::G_M_xmm
        | _op @ VEXOperandCode::G_U_xmm
        | _op @ VEXOperandCode::G_E_xmm
        | _op @ VEXOperandCode::G_E_xmm_imm8 => {
            if instruction.vex_reg.num != 0 {
                instruction.opcode = Opcode::Invalid;
                return Err(DecodeError::InvalidOperand);
            }
            let modrm = read_modrm(bytes, length)?;
            instruction.modrm_rrr = RegSpec::from_parts(
                (modrm >> 3) & 7,
                instruction.prefixes.vex().r(),
                RegisterBank::X,
            );
            let mem_oper = read_E_xmm(bytes, instruction, modrm, length)?;
            instruction.operands[0] = OperandSpec::RegRRR;
            instruction.operands[1] = mem_oper;
            Ok(())
        }

        _op @ VEXOperandCode::E_G_ymm
        | _op @ VEXOperandCode::U_G_ymm
        | _op @ VEXOperandCode::M_G_ymm => {
            if instruction.vex_reg.num != 0 {
                instruction.opcode = Opcode::Invalid;
                return Err(DecodeError::InvalidOperand);
            }
            let modrm = read_modrm(bytes, length)?;
            instruction.modrm_rrr = RegSpec::from_parts(
                (modrm >> 3) & 7,
                instruction.prefixes.vex().r(),
                RegisterBank::Y,
            );
            let mem_oper = read_E_ymm(bytes, instruction, modrm, length)?;
            instruction.operands[0] = mem_oper;
            instruction.operands[1] = OperandSpec::RegRRR;
            Ok(())
        }

        _op @ VEXOperandCode::G_M_ymm
        | _op @ VEXOperandCode::G_U_ymm
        | _op @ VEXOperandCode::G_E_ymm => {
            if instruction.vex_reg.num != 0 {
                instruction.opcode = Opcode::Invalid;
                return Err(DecodeError::InvalidOperand);
            }
            let modrm = read_modrm(bytes, length)?;
            instruction.modrm_rrr = RegSpec::from_parts(
                (modrm >> 3) & 7,
                instruction.prefixes.vex().r(),
                RegisterBank::Y,
            );
            let mem_oper = read_E_ymm(bytes, instruction, modrm, length)?;
            instruction.operands[0] = OperandSpec::RegRRR;
            instruction.operands[1] = mem_oper;
            Ok(())
        }
        _op @ VEXOperandCode::G_V_E_ymm | _op @ VEXOperandCode::G_V_M_ymm => {
            let modrm = read_modrm(bytes, length)?;
            instruction.modrm_rrr = RegSpec::from_parts(
                (modrm >> 3) & 7,
                instruction.prefixes.vex().r(),
                RegisterBank::Y,
            );
            instruction.vex_reg.bank = RegisterBank::Y;
            let mem_oper = read_E_ymm(bytes, instruction, modrm, length)?;
            instruction.operands[0] = OperandSpec::RegRRR;
            instruction.operands[1] = OperandSpec::RegVex;
            instruction.operands[2] = mem_oper;
            Ok(())
        }
        _op @ VEXOperandCode::E_V_G_ymm | _op @ VEXOperandCode::M_V_G_ymm => {
            let modrm = read_modrm(bytes, length)?;
            instruction.modrm_rrr = RegSpec::from_parts(
                (modrm >> 3) & 7,
                instruction.prefixes.vex().r(),
                RegisterBank::Y,
            );
            instruction.vex_reg.bank = RegisterBank::Y;
            let mem_oper = read_E_ymm(bytes, instruction, modrm, length)?;
            instruction.operands[0] = mem_oper;
            instruction.operands[1] = OperandSpec::RegVex;
            instruction.operands[2] = OperandSpec::RegRRR;
            Ok(())
        }
        _op @ VEXOperandCode::G_V_M_xmm | _op @ VEXOperandCode::G_V_E_xmm => {
            let modrm = read_modrm(bytes, length)?;
            instruction.modrm_rrr = RegSpec::from_parts(
                (modrm >> 3) & 7,
                instruction.prefixes.vex().r(),
                RegisterBank::X,
            );
            let mem_oper = read_E_xmm(bytes, instruction, modrm, length)?;
            instruction.operands[0] = OperandSpec::RegRRR;
            instruction.operands[1] = OperandSpec::RegVex;
            instruction.operands[2] = mem_oper;
            Ok(())
        }

        _op @ VEXOperandCode::E_V_G_xmm | _op @ VEXOperandCode::M_V_G_xmm => {
            let modrm = read_modrm(bytes, length)?;
            instruction.modrm_rrr = RegSpec::from_parts(
                (modrm >> 3) & 7,
                instruction.prefixes.vex().r(),
                RegisterBank::X,
            );
            let mem_oper = read_E_xmm(bytes, instruction, modrm, length)?;
            instruction.operands[0] = mem_oper;
            instruction.operands[1] = OperandSpec::RegVex;
            instruction.operands[2] = OperandSpec::RegRRR;
            Ok(())
        }

        VEXOperandCode::G_Ex_V_xmm => {
            let modrm = read_modrm(bytes, length)?;
            instruction.modrm_rrr = RegSpec::from_parts(
                (modrm >> 3) & 7,
                instruction.prefixes.vex().r(),
                RegisterBank::X,
            );
            let mem_oper = read_E_xmm(bytes, instruction, modrm, length)?;
            instruction.sib_index.bank = RegisterBank::X;
            instruction.operands[0] = OperandSpec::RegRRR;
            instruction.operands[1] = mem_oper;
            instruction.operands[2] = OperandSpec::RegVex;
            Ok(())
        }
        VEXOperandCode::G_Ey_V_ymm => {
            let modrm = read_modrm(bytes, length)?;
            instruction.modrm_rrr = RegSpec::from_parts(
                (modrm >> 3) & 7,
                instruction.prefixes.vex().r(),
                RegisterBank::Y,
            );
            let mem_oper = read_E_ymm(bytes, instruction, modrm, length)?;
            instruction.vex_reg.bank = RegisterBank::Y;
            instruction.sib_index.bank = RegisterBank::Y;
            instruction.operands[0] = OperandSpec::RegRRR;
            instruction.operands[1] = mem_oper;
            instruction.operands[2] = OperandSpec::RegVex;
            Ok(())
        }

        VEXOperandCode::E_xmm_G_ymm_imm8
        | VEXOperandCode::G_E_ymm_imm8
        | VEXOperandCode::G_xmm_E_xmm
        | VEXOperandCode::G_xmm_E_ymm
        | VEXOperandCode::G_ymm_E_xmm
        | VEXOperandCode::G_ymm_E_ymm
        | VEXOperandCode::G_V_E_xmm_imm8
        | VEXOperandCode::G_V_E_xmm_xmm4
        | VEXOperandCode::G_V_E_ymm_imm8
        | VEXOperandCode::G_V_E_ymm_ymm4
        | VEXOperandCode::V_xmm_G_ymm_E_ymm_imm8
        | VEXOperandCode::V_ymm_G_ymm_E_xmm_imm8
        | VEXOperandCode::Eq_G_xmm
        | VEXOperandCode::Ed_G_xmm
        | VEXOperandCode::G_xmm_Ed
        | VEXOperandCode::G_xmm_Eq
        | VEXOperandCode::G_V_ymm_E_xmm
        | VEXOperandCode::G_V_xmm_Ew_imm8 => {
            Err(DecodeError::IncompleteDecoder) // :)
        }
    }
}

fn read_vex_instruction<T: Iterator<Item = u8>>(
    opcode_map: VEXOpcodeMap,
    bytes: &mut T,
    instruction: &mut Instruction,
    length: &mut u8,
    p: VEXOpcodePrefix,
) -> Result<(), DecodeError> {
    let opc = bytes.next().ok_or(DecodeError::ExhaustedInput)?;
    *length += 1;

    // the name of this bit is `L` in the documentation, so use the same name here.
    #[allow(non_snake_case)]
    let L = instruction.prefixes.vex().l();

    //    println!("reading vex instruction from opcode prefix {:?}, L: {}, opc: {:#x}, map:{:?}", p, L, opc, opcode_map);
    //    println!("w? {}", instruction.prefixes.vex().w());

    // several combinations simply have no instructions. check for those first.
    let (opcode, operand_code) = match opcode_map {
        VEXOpcodeMap::Map0F => {
            match p {
                VEXOpcodePrefix::None => {
                    match opc {
                        0x10 => (
                            Opcode::VMOVUPS,
                            if L {
                                VEXOperandCode::G_E_ymm
                            } else {
                                VEXOperandCode::G_E_xmm
                            },
                        ),
                        0x11 => (
                            Opcode::VMOVUPS,
                            if L {
                                VEXOperandCode::E_G_ymm
                            } else {
                                VEXOperandCode::E_G_xmm
                            },
                        ),
                        // ugh
                        //                        0x12 => (Opcode::VMOVHLPS, ..),
                        //                        0x12 => (Opcode::VMOVLPS, ..),
                        0x13 => (
                            Opcode::VMOVLPS,
                            if L {
                                instruction.opcode = Opcode::Invalid;
                                return Err(DecodeError::InvalidOpcode);
                            } else {
                                VEXOperandCode::M_G_xmm
                            },
                        ),
                        0x14 => (
                            Opcode::VUNPCKLPS,
                            if L {
                                VEXOperandCode::G_V_E_ymm
                            } else {
                                VEXOperandCode::G_V_E_xmm
                            },
                        ),
                        0x15 => (
                            Opcode::VUNPCKHPS,
                            if L {
                                VEXOperandCode::G_V_E_ymm
                            } else {
                                VEXOperandCode::G_V_E_xmm
                            },
                        ),
                        // ugh
                        //                        0x16 => (Opcode::VMOVHPS, ..),
                        //                        0x16 => (Opcode::VMOVLHPS, ..),
                        0x17 => (
                            Opcode::VMOVHPS,
                            if L {
                                instruction.opcode = Opcode::Invalid;
                                return Err(DecodeError::InvalidOpcode);
                            } else {
                                VEXOperandCode::M_G_xmm
                            },
                        ),
                        0x28 => (
                            Opcode::VMOVAPS,
                            if L {
                                VEXOperandCode::G_E_ymm
                            } else {
                                VEXOperandCode::G_E_xmm
                            },
                        ),
                        0x29 => (
                            Opcode::VMOVAPS,
                            if L {
                                VEXOperandCode::E_G_ymm
                            } else {
                                VEXOperandCode::E_G_xmm
                            },
                        ),
                        0x2B => (
                            Opcode::VMOVNTPS,
                            if L {
                                VEXOperandCode::M_G_ymm
                            } else {
                                VEXOperandCode::M_G_xmm
                            },
                        ),
                        0x2e => (Opcode::VUCOMISS, VEXOperandCode::G_E_xmm),
                        0x2f => (Opcode::VCOMISS, VEXOperandCode::G_E_xmm),
                        0x50 => (
                            Opcode::VMOVMSKPS,
                            if L {
                                VEXOperandCode::U_G_ymm
                            } else {
                                VEXOperandCode::U_G_xmm
                            },
                        ),
                        0x51 => (
                            Opcode::VSQRTPS,
                            if L {
                                VEXOperandCode::G_E_ymm
                            } else {
                                VEXOperandCode::G_E_xmm
                            },
                        ),
                        0x52 => (
                            Opcode::VRSQRTPS,
                            if L {
                                VEXOperandCode::G_E_ymm
                            } else {
                                VEXOperandCode::G_E_xmm
                            },
                        ),
                        0x53 => (
                            Opcode::VRCPPS,
                            if L {
                                VEXOperandCode::G_E_ymm
                            } else {
                                VEXOperandCode::G_E_xmm
                            },
                        ),
                        0x57 => (
                            Opcode::VXORPS,
                            if L {
                                VEXOperandCode::G_V_E_ymm
                            } else {
                                VEXOperandCode::G_V_E_xmm
                            },
                        ),
                        0x58 => (
                            Opcode::VADDPS,
                            if L {
                                VEXOperandCode::G_V_E_ymm
                            } else {
                                VEXOperandCode::G_V_E_xmm
                            },
                        ),
                        0x59 => (
                            Opcode::VMULPS,
                            if L {
                                VEXOperandCode::G_V_E_ymm
                            } else {
                                VEXOperandCode::G_V_E_xmm
                            },
                        ),
                        0x5A => (
                            Opcode::VCVTPS2PD,
                            if L {
                                VEXOperandCode::G_E_ymm
                            } else {
                                VEXOperandCode::G_E_xmm
                            },
                        ),
                        0x5B => (
                            Opcode::VCVTDQ2PS,
                            if L {
                                VEXOperandCode::G_E_ymm
                            } else {
                                VEXOperandCode::G_E_xmm
                            },
                        ),
                        0x5C => (
                            Opcode::VSUBPS,
                            if L {
                                VEXOperandCode::G_V_E_ymm
                            } else {
                                VEXOperandCode::G_V_E_xmm
                            },
                        ),
                        0x5D => (
                            Opcode::VMINPS,
                            if L {
                                VEXOperandCode::G_V_E_ymm
                            } else {
                                VEXOperandCode::G_V_E_xmm
                            },
                        ),
                        0x5E => (
                            Opcode::VDIVPS,
                            if L {
                                VEXOperandCode::G_V_E_ymm
                            } else {
                                VEXOperandCode::G_V_E_xmm
                            },
                        ),
                        0x5F => (
                            Opcode::VMAXPS,
                            if L {
                                VEXOperandCode::G_V_E_ymm
                            } else {
                                VEXOperandCode::G_V_E_xmm
                            },
                        ),
                        0x77 => (Opcode::VZEROUPPER, VEXOperandCode::Nothing),
                        0xC2 => (
                            Opcode::VCMPPS,
                            if L {
                                VEXOperandCode::G_V_E_ymm_imm8
                            } else {
                                VEXOperandCode::G_V_E_xmm_imm8
                            },
                        ),
                        0xC6 => (
                            Opcode::VSHUFPS,
                            if L {
                                VEXOperandCode::G_V_E_ymm_imm8
                            } else {
                                VEXOperandCode::G_V_E_xmm_imm8
                            },
                        ),
                        _ => {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        }
                    }
                }
                VEXOpcodePrefix::Prefix66 => match opc {
                    0x0a => (Opcode::VROUNDSS, VEXOperandCode::G_V_E_xmm_imm8),
                    0x0b => (Opcode::VROUNDSD, VEXOperandCode::G_V_E_xmm_imm8),
                    0x10 => (
                        Opcode::VMOVUPD,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x11 => (
                        Opcode::VMOVUPD,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x12 => (
                        Opcode::VMOVLPD,
                        if L {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        } else {
                            VEXOperandCode::G_V_M_xmm
                        },
                    ),
                    0x13 => (
                        Opcode::VMOVLPD,
                        if L {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        } else {
                            VEXOperandCode::M_G_xmm
                        },
                    ),
                    0x14 => (
                        Opcode::VUNPCKLPD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x15 => (
                        Opcode::VUNPCKHPD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x16 => (
                        Opcode::VMOVHPD,
                        if L {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        } else {
                            VEXOperandCode::G_V_M_xmm
                        },
                    ),
                    0x17 => (
                        Opcode::VMOVHPD,
                        if L {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        } else {
                            VEXOperandCode::M_G_xmm
                        },
                    ),
                    0x28 => (
                        Opcode::VMOVAPD,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x29 => (
                        Opcode::VMOVAPD,
                        if L {
                            VEXOperandCode::E_G_ymm
                        } else {
                            VEXOperandCode::E_G_xmm
                        },
                    ),
                    0x2B => (
                        Opcode::VMOVNTPD,
                        if L {
                            VEXOperandCode::M_G_ymm
                        } else {
                            VEXOperandCode::M_G_xmm
                        },
                    ),
                    0x2e => (Opcode::VUCOMISD, VEXOperandCode::G_E_xmm),
                    0x2f => (Opcode::VCOMISD, VEXOperandCode::G_E_xmm),
                    0x50 => (
                        Opcode::VMOVMSKPD,
                        if L {
                            VEXOperandCode::G_U_ymm
                        } else {
                            VEXOperandCode::G_U_xmm
                        },
                    ),
                    0x51 => (
                        Opcode::VSQRTPD,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x57 => (
                        Opcode::VXORPD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x58 => (
                        Opcode::VADDPD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x59 => (
                        Opcode::VMULPD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x5A => (
                        Opcode::VCVTPD2PS,
                        if L {
                            VEXOperandCode::G_xmm_E_ymm
                        } else {
                            VEXOperandCode::G_xmm_E_xmm
                        },
                    ),
                    0x5B => (
                        Opcode::VCVTPS2DQ,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x5C => (
                        Opcode::VSUBPD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x5D => (
                        Opcode::VMINPD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x5E => (
                        Opcode::VDIVPD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x5F => (
                        Opcode::VMAXPD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x60 => (
                        Opcode::VPUNPCKLBW,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x61 => (
                        Opcode::VPUNPCKLWD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x62 => (
                        Opcode::VPUNPCKLDQ,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x63 => (
                        Opcode::VPACKSSWB,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x64 => (
                        Opcode::VPCMPGTB,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x65 => (
                        Opcode::VPCMPGTW,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x66 => (
                        Opcode::VPCMPGTD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x67 => (
                        Opcode::VPACKUSWB,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x68 => (
                        Opcode::VPUNPCKHBW,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x69 => (
                        Opcode::VPUNPCKHWD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x6A => (
                        Opcode::VPUNPCKHDQ,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x6B => (
                        Opcode::VPACKSSDW,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x6C => (
                        Opcode::VPUNPCKLQDQ,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x6D => (
                        Opcode::VPUNPCKHQDQ,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x6E => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VMOVQ,
                                if L {
                                    instruction.opcode = Opcode::Invalid;
                                    return Err(DecodeError::InvalidOpcode);
                                } else {
                                    VEXOperandCode::G_xmm_Eq
                                },
                            )
                        } else {
                            (
                                Opcode::VMOVD,
                                if L {
                                    instruction.opcode = Opcode::Invalid;
                                    return Err(DecodeError::InvalidOpcode);
                                } else {
                                    VEXOperandCode::G_xmm_Ed
                                },
                            )
                        }
                    }
                    0x6F => (
                        Opcode::VMOVDQA,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x70 => (
                        Opcode::VPSHUFD,
                        if L {
                            VEXOperandCode::G_E_ymm_imm8
                        } else {
                            VEXOperandCode::G_E_xmm_imm8
                        },
                    ),
                    0x71 => (
                        Opcode::Invalid,
                        if L {
                            VEXOperandCode::VPS_71_L
                        } else {
                            VEXOperandCode::VPS_71
                        },
                    ),
                    0x72 => (
                        Opcode::Invalid,
                        if L {
                            VEXOperandCode::VPS_72_L
                        } else {
                            VEXOperandCode::VPS_72
                        },
                    ),
                    0x73 => (
                        Opcode::Invalid,
                        if L {
                            VEXOperandCode::VPS_73_L
                        } else {
                            VEXOperandCode::VPS_73
                        },
                    ),
                    0x74 => (
                        Opcode::VPCMPEQB,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x75 => (
                        Opcode::VPCMPEQW,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x76 => (
                        Opcode::VPCMPEQD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x7C => (
                        Opcode::VHADDPD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x7D => (
                        Opcode::VHSUBPD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x7E => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VMOVQ,
                                if L {
                                    instruction.opcode = Opcode::Invalid;
                                    return Err(DecodeError::InvalidOpcode);
                                } else {
                                    VEXOperandCode::Eq_G_xmm
                                },
                            )
                        } else {
                            (
                                Opcode::VMOVD,
                                if L {
                                    instruction.opcode = Opcode::Invalid;
                                    return Err(DecodeError::InvalidOpcode);
                                } else {
                                    VEXOperandCode::Ed_G_xmm
                                },
                            )
                        }
                    }
                    0x7F => (
                        Opcode::VMOVDQA,
                        if L {
                            VEXOperandCode::E_G_ymm
                        } else {
                            VEXOperandCode::E_G_xmm
                        },
                    ),
                    0xC2 => (
                        Opcode::VCMPPD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xC4 => (
                        Opcode::VPINSRW,
                        if L {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        } else {
                            VEXOperandCode::G_V_xmm_Ew_imm8
                        },
                    ),
                    0xC5 => (
                        Opcode::VPEXTRW,
                        if L {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        } else {
                            VEXOperandCode::U_G_xmm_imm8
                        },
                    ),
                    0xC6 => (
                        Opcode::VSHUFPD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xD0 => (
                        Opcode::VADDSUBPD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xD1 => (
                        Opcode::VPSRLW,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xD2 => (
                        Opcode::VPSRLD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xD3 => (
                        Opcode::VPSRLQ,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xD4 => (
                        Opcode::VPADDQ,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xD5 => (
                        Opcode::VPMULLW,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xD6 => (
                        Opcode::VMOVQ,
                        if L {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0xD7 => (
                        Opcode::VPMOVMSKB,
                        if L {
                            VEXOperandCode::U_G_ymm
                        } else {
                            VEXOperandCode::U_G_xmm
                        },
                    ),
                    0xD8 => (
                        Opcode::VPSUBUSB,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xD9 => (
                        Opcode::VPSUBUSW,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xDB => (
                        Opcode::VPAND,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xDC => (
                        Opcode::VPADDUSB,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        },
                    ),
                    0xDD => (
                        Opcode::VPADDUSW,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xDF => (
                        Opcode::VPANDN,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xE0 => (
                        Opcode::VPAVGB,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xE1 => (
                        Opcode::VPSRAW,
                        if L {
                            VEXOperandCode::G_V_ymm_E_xmm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xE2 => (
                        Opcode::VPSRAD,
                        if L {
                            VEXOperandCode::G_V_ymm_E_xmm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xE3 => (
                        Opcode::VPAVGW,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xE4 => (
                        Opcode::VPMULHUW,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xE5 => (
                        Opcode::VPMULHW,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xE6 => (
                        Opcode::VCVTTPD2DQ,
                        if L {
                            VEXOperandCode::G_xmm_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0xE7 => (
                        Opcode::VMOVNTDQ,
                        if L {
                            VEXOperandCode::M_G_ymm
                        } else {
                            VEXOperandCode::M_G_xmm
                        },
                    ),
                    0xE8 => (
                        Opcode::VPSUBSB,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xE9 => (
                        Opcode::VPSUBSW,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xEB => (
                        Opcode::VPOR,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xEC => (
                        Opcode::VPADDSB,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xED => (
                        Opcode::VPADDSW,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xEE => (
                        Opcode::VPMAXSW,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xEF => (
                        Opcode::VPXOR,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xF1 => (
                        Opcode::VPSLLW,
                        if L {
                            VEXOperandCode::G_V_ymm_E_xmm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xF2 => (
                        Opcode::VPSLLD,
                        if L {
                            VEXOperandCode::G_V_ymm_E_xmm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xF3 => (
                        Opcode::VPSLLQ,
                        if L {
                            VEXOperandCode::G_V_ymm_E_xmm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xF4 => (
                        Opcode::VPMULUDQ,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xF5 => (
                        Opcode::VPMADDWD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xF6 => (
                        Opcode::VPSADBW,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xF7 => (
                        Opcode::VMASKMOVDQU,
                        if L {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0xF8 => (
                        Opcode::VPSUBB,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xF9 => (
                        Opcode::VPSUBW,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xFA => (
                        Opcode::VPSUBD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xFB => (
                        Opcode::VPSUBQ,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xFC => (
                        Opcode::VPADDB,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xFD => (
                        Opcode::VPADDW,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xFE => (
                        Opcode::VPADDD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    _ => {
                        instruction.opcode = Opcode::Invalid;
                        return Err(DecodeError::InvalidOpcode);
                    }
                },
                VEXOpcodePrefix::PrefixF2 => {
                    match opc {
                        0x10 => (Opcode::VMOVSD, VEXOperandCode::VMOVSD_10),
                        0x11 => (Opcode::VMOVSD, VEXOperandCode::VMOVSD_11),
                        0x12 => (
                            Opcode::VMOVDDUP,
                            if L {
                                VEXOperandCode::G_E_ymm
                            } else {
                                VEXOperandCode::G_E_xmm
                            },
                        ),
                        0x2a => (
                            Opcode::VCVTSI2SD,
                            if instruction.prefixes.vex().w() {
                                VEXOperandCode::G_V_E_xmm // 64-bit last operand
                            } else {
                                VEXOperandCode::G_V_E_xmm // 32-bit last operand
                            },
                        ),
                        0x2c => (
                            Opcode::VCVTTSD2SI,
                            if instruction.prefixes.vex().w() {
                                VEXOperandCode::G_E_xmm // 64-bit
                            } else {
                                VEXOperandCode::G_E_xmm // 32-bit
                            },
                        ),
                        0x2d => (
                            Opcode::VCVTSD2SI,
                            if instruction.prefixes.vex().w() {
                                VEXOperandCode::G_E_xmm // 64-bit
                            } else {
                                VEXOperandCode::G_E_xmm // 32-bit
                            },
                        ),
                        0x51 => (Opcode::VSQRTSD, VEXOperandCode::G_V_E_xmm),
                        0x58 => (Opcode::VADDSD, VEXOperandCode::G_V_E_xmm),
                        0x59 => (Opcode::VMULSD, VEXOperandCode::G_V_E_xmm),
                        0x5a => (Opcode::CVTSD2SS, VEXOperandCode::G_V_E_xmm),
                        0x5c => (Opcode::VSUBSD, VEXOperandCode::G_V_E_xmm),
                        0x5d => (Opcode::VMINSD, VEXOperandCode::G_V_E_xmm),
                        0x5e => (Opcode::VDIVSD, VEXOperandCode::G_V_E_xmm),
                        0x5f => (Opcode::VMAXSD, VEXOperandCode::G_V_E_xmm),
                        0x70 => (
                            Opcode::VPSHUFLW,
                            if L {
                                VEXOperandCode::G_E_ymm_imm8
                            } else {
                                VEXOperandCode::G_E_xmm_imm8
                            },
                        ),
                        0x7c => (
                            Opcode::VHADDPS,
                            if L {
                                VEXOperandCode::G_V_E_ymm
                            } else {
                                VEXOperandCode::G_V_E_xmm
                            },
                        ),
                        0x7d => (
                            Opcode::VHSUBPS,
                            if L {
                                VEXOperandCode::G_V_E_ymm
                            } else {
                                VEXOperandCode::G_V_E_xmm
                            },
                        ),
                        0xc2 => (Opcode::VCMPSD, VEXOperandCode::G_V_E_xmm_imm8),
                        0xd0 => (
                            Opcode::VADDSUBPS,
                            if L {
                                VEXOperandCode::G_V_E_ymm
                            } else {
                                VEXOperandCode::G_V_E_xmm
                            },
                        ),
                        0xe6 => (
                            Opcode::VCVTPD2DQ,
                            if L {
                                VEXOperandCode::G_xmm_E_ymm
                            } else {
                                VEXOperandCode::G_xmm_E_xmm
                            },
                        ),
                        0xf0 => (
                            Opcode::VLDDQU,
                            if L {
                                VEXOperandCode::G_M_ymm
                            } else {
                                VEXOperandCode::G_M_ymm
                            },
                        ),
                        _ => {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        }
                    }
                }
                VEXOpcodePrefix::PrefixF3 => {
                    match opc {
                        0x10 => (Opcode::VMOVSS, VEXOperandCode::VMOVSS_10),
                        0x11 => (Opcode::VMOVSS, VEXOperandCode::VMOVSS_11),
                        0x12 => (
                            Opcode::VMOVSLDUP,
                            if L {
                                VEXOperandCode::G_E_ymm
                            } else {
                                VEXOperandCode::G_E_xmm
                            },
                        ),
                        0x16 => (
                            Opcode::VMOVSHDUP,
                            if L {
                                VEXOperandCode::G_E_ymm
                            } else {
                                VEXOperandCode::G_E_xmm
                            },
                        ),
                        0x2a => (
                            Opcode::VCVTSI2SS,
                            if instruction.prefixes.vex().w() {
                                VEXOperandCode::G_V_E_xmm // 64-bit last operand
                            } else {
                                VEXOperandCode::G_V_E_xmm // 32-bit last operand
                            },
                        ),
                        0x2c => (
                            Opcode::VCVTTSS2SI,
                            if instruction.prefixes.vex().w() {
                                VEXOperandCode::G_E_xmm // 64-bit
                            } else {
                                VEXOperandCode::G_E_xmm // 32-bit
                            },
                        ),
                        0x2d => (
                            Opcode::VCVTSD2SI,
                            if instruction.prefixes.vex().w() {
                                VEXOperandCode::G_E_xmm // 64-bit
                            } else {
                                VEXOperandCode::G_E_xmm // 32-bit
                            },
                        ),
                        0x51 => (Opcode::VSQRTSS, VEXOperandCode::G_V_E_xmm),
                        0x52 => (Opcode::VRSQRTSS, VEXOperandCode::G_V_E_xmm),
                        0x53 => (Opcode::VRCPSS, VEXOperandCode::G_V_E_xmm),
                        0x58 => (Opcode::VADDSS, VEXOperandCode::G_V_E_xmm),
                        0x59 => (Opcode::VMULSS, VEXOperandCode::G_V_E_xmm),
                        0x5a => (Opcode::VCVTSS2SD, VEXOperandCode::G_V_E_xmm),
                        0x5b => (
                            Opcode::VCVTTPS2DQ,
                            if L {
                                VEXOperandCode::G_ymm_E_ymm
                            } else {
                                VEXOperandCode::G_xmm_E_xmm
                            },
                        ),
                        0x5c => (Opcode::VSUBSS, VEXOperandCode::G_V_E_xmm),
                        0x5d => (Opcode::VMINSS, VEXOperandCode::G_V_E_xmm),
                        0x5e => (Opcode::VDIVSS, VEXOperandCode::G_V_E_xmm),
                        0x5f => (Opcode::VMAXSS, VEXOperandCode::G_V_E_xmm),
                        0x6f => (
                            Opcode::VMOVDQU,
                            if L {
                                VEXOperandCode::G_E_ymm
                            } else {
                                VEXOperandCode::G_E_xmm
                            },
                        ),
                        0x70 => (
                            Opcode::VMOVSHDUP,
                            if L {
                                VEXOperandCode::G_E_ymm_imm8
                            } else {
                                VEXOperandCode::G_E_xmm_imm8
                            },
                        ),
                        0x7e => (
                            Opcode::VMOVQ,
                            if L {
                                instruction.opcode = Opcode::Invalid;
                                return Err(DecodeError::InvalidOpcode);
                            } else {
                                VEXOperandCode::G_E_xmm
                            },
                        ),
                        0x7f => (
                            Opcode::VMOVDQU,
                            if L {
                                VEXOperandCode::E_G_ymm
                            } else {
                                VEXOperandCode::E_G_xmm
                            },
                        ),
                        0xc2 => (Opcode::VCMPSS, VEXOperandCode::G_V_E_xmm_imm8),
                        0xe6 => (
                            Opcode::VCVTDQ2PD,
                            if L {
                                VEXOperandCode::G_ymm_E_xmm
                            } else {
                                VEXOperandCode::G_xmm_E_xmm
                            },
                        ),
                        _ => {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        }
                    }
                }
            }
        }
        VEXOpcodeMap::Map0F38 => {
            // TODO: verify rejecting invalid W bit
            if let VEXOpcodePrefix::Prefix66 = p {
                // possibly valid!
                match opc {
                    0x00 => (
                        Opcode::VPSHUFB,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x01 => (
                        Opcode::VPHADDW,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x02 => (
                        Opcode::VPHADDD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x03 => (
                        Opcode::VPHADDSW,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x04 => (
                        Opcode::VPHADDUBSW,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x05 => (
                        Opcode::VPHSUBW,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x06 => (
                        Opcode::VPHSUBD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x07 => (
                        Opcode::VPHSUBSW,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x08 => (
                        Opcode::VPSIGNB,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x09 => (
                        Opcode::VPSIGNW,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x0A => (
                        Opcode::VPSIGND,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x0B => (
                        Opcode::VPMULHRSW,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x0C => (
                        Opcode::VPERMILPS,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x0D => (
                        Opcode::VPERMILPD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x0E => (
                        Opcode::VTESTPS,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x0F => (
                        Opcode::VTESTPD,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x13 => (
                        Opcode::VCVTPH2PS,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x16 => (
                        Opcode::VPERMPS,
                        if L {
                            VEXOperandCode::G_V_E_xmm
                        } else {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        },
                    ),
                    0x17 => (
                        Opcode::VPTEST,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x18 => (
                        Opcode::VBROADCASTSS,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x19 => (
                        Opcode::VBROADCASTSD,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x1A => (
                        Opcode::VBROADCASTF128,
                        if L {
                            VEXOperandCode::G_M_ymm
                        } else {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        },
                    ),
                    0x1C => (
                        Opcode::VPABSB,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x1D => (
                        Opcode::VPABSW,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x1E => (
                        Opcode::VPABSD,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x20 => (
                        Opcode::VPMOVSXBW,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x21 => (
                        Opcode::VPMOVSXBD,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x22 => (
                        Opcode::VPMOVSXBQ,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x23 => (
                        Opcode::VPMOVSXWD,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x24 => (
                        Opcode::VPMOVSXWQ,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x25 => (
                        Opcode::VPMOVSXDQ,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x28 => (
                        Opcode::VPMULDQ,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x29 => (
                        Opcode::VPCMPEQQ,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x2A => (
                        Opcode::VMOVNTDQA,
                        if L {
                            VEXOperandCode::G_M_ymm
                        } else {
                            VEXOperandCode::G_M_xmm
                        },
                    ),
                    0x2C => (
                        Opcode::VMASKMOVPS,
                        if L {
                            VEXOperandCode::G_V_M_ymm
                        } else {
                            VEXOperandCode::G_V_M_xmm
                        },
                    ),
                    0x2D => (
                        Opcode::VMASKMOVPD,
                        if L {
                            VEXOperandCode::G_V_M_ymm
                        } else {
                            VEXOperandCode::G_V_M_xmm
                        },
                    ),
                    0x2E => (
                        Opcode::VMASKMOVPS,
                        if L {
                            VEXOperandCode::M_V_G_ymm
                        } else {
                            VEXOperandCode::M_V_G_xmm
                        },
                    ),
                    0x2F => (
                        Opcode::VMASKMOVPD,
                        if L {
                            VEXOperandCode::M_V_G_ymm
                        } else {
                            VEXOperandCode::M_V_G_xmm
                        },
                    ),
                    0x30 => (
                        Opcode::VPMOVZXBW,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x31 => (
                        Opcode::VPMOVZXBD,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x32 => (
                        Opcode::VPMOVZXBQ,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x33 => (
                        Opcode::VPMOVZXWD,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x34 => (
                        Opcode::VPMOVZXWQ,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x35 => (
                        Opcode::VPMOVZXDQ,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        },
                    ),
                    0x36 => (
                        Opcode::VPERMD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        },
                    ),
                    0x37 => (
                        Opcode::VPCMPGTQ,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x39 => (
                        Opcode::VPMINSD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x3B => (
                        Opcode::VPMINUD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x3C => (
                        Opcode::VPMAXSB,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x3D => (
                        Opcode::VPMAXSD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x3F => (
                        Opcode::VPMAXUD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x40 => (
                        Opcode::VPMULLD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x41 => (
                        Opcode::VPHMINPOSUW,
                        if L {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x45 => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VPSRLVQ,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_xmm
                                },
                            )
                        } else {
                            (
                                Opcode::VPSRLVD,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_xmm
                                },
                            )
                        }
                    }
                    0x46 => (
                        Opcode::VPSRAVD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x47 => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VPSLLVQ,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_xmm
                                },
                            )
                        } else {
                            (
                                Opcode::VPSLLVD,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_xmm
                                },
                            )
                        }
                    }
                    0x58 => (
                        Opcode::VPBROADCASTD,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x59 => (
                        Opcode::VPBROADCASTQ,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0x5A => (
                        Opcode::VBROADCASTI128,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        },
                    ),
                    0x78 => (
                        Opcode::VPBROADCASTB,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_ymm
                        },
                    ),
                    0x79 => (
                        Opcode::VPBROADCASTW,
                        if L {
                            VEXOperandCode::G_E_ymm
                        } else {
                            VEXOperandCode::G_E_ymm
                        },
                    ),
                    0x8C => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VPMASKMOVD,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        } else {
                            (
                                Opcode::VPMASKMOVQ,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        }
                    }
                    0x8E => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VPMASKMOVD,
                                if L {
                                    VEXOperandCode::E_V_G_ymm
                                } else {
                                    VEXOperandCode::E_V_G_xmm
                                },
                            )
                        } else {
                            (
                                Opcode::VPMASKMOVQ,
                                if L {
                                    VEXOperandCode::E_V_G_ymm
                                } else {
                                    VEXOperandCode::E_V_G_xmm
                                },
                            )
                        }
                    }
                    0x90 => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VPGATHERDQ,
                                if L {
                                    VEXOperandCode::G_Ey_V_ymm
                                } else {
                                    VEXOperandCode::G_Ex_V_xmm
                                },
                            )
                        } else {
                            (
                                Opcode::VPGATHERDD,
                                if L {
                                    VEXOperandCode::G_Ey_V_ymm
                                } else {
                                    VEXOperandCode::G_Ex_V_xmm
                                },
                            )
                        }
                    }
                    0x91 => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VPGATHERQQ,
                                if L {
                                    VEXOperandCode::G_Ey_V_ymm
                                } else {
                                    VEXOperandCode::G_Ex_V_xmm
                                },
                            )
                        } else {
                            (
                                Opcode::VPGATHERQD,
                                if L {
                                    VEXOperandCode::G_Ey_V_ymm
                                } else {
                                    VEXOperandCode::G_Ex_V_xmm
                                },
                            )
                        }
                    }
                    0x92 => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VGATHERDPD,
                                if L {
                                    VEXOperandCode::G_E_ymm_imm8
                                } else {
                                    VEXOperandCode::G_E_xmm_imm8
                                },
                            )
                        } else {
                            (
                                Opcode::VGATHERDPS,
                                if L {
                                    VEXOperandCode::G_E_ymm_imm8
                                } else {
                                    VEXOperandCode::G_E_xmm_imm8
                                },
                            )
                        }
                    }
                    0x93 => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VGATHERQPD,
                                if L {
                                    VEXOperandCode::G_E_ymm_imm8
                                } else {
                                    VEXOperandCode::G_E_xmm_imm8
                                },
                            )
                        } else {
                            (
                                Opcode::VGATHERQPS,
                                if L {
                                    VEXOperandCode::G_E_ymm_imm8
                                } else {
                                    VEXOperandCode::G_E_xmm_imm8
                                },
                            )
                        }
                    }
                    0x96 => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFMADDSUB132PD,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        } else {
                            (
                                Opcode::VFMADDSUB132PS,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        }
                    }
                    0x97 => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFMSUBADD132PD,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        } else {
                            (
                                Opcode::VFMSUBADD132PS,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        }
                    }
                    0x98 => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFMADD132PD,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        } else {
                            (
                                Opcode::VFMADD132PS,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        }
                    }
                    0x99 => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFMADD132SD,
                                VEXOperandCode::G_V_E_xmm, /* 64bit */
                            )
                        } else {
                            (
                                Opcode::VFMADD132SS,
                                VEXOperandCode::G_V_E_xmm, /* 64bit */
                            )
                        }
                    }
                    0x9A => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFMSUB132PD,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        } else {
                            (
                                Opcode::VFMSUB132PS,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        }
                    }
                    0x9B => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFMSUB132SD,
                                VEXOperandCode::G_V_E_xmm, /* 64bit */
                            )
                        } else {
                            (
                                Opcode::VFMSUB132SS,
                                VEXOperandCode::G_V_E_xmm, /* 64bit */
                            )
                        }
                    }
                    0x9C => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFNMADD132PD,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        } else {
                            (
                                Opcode::VFNMADD132PS,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        }
                    }
                    0x9D => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFNMADD132SD,
                                VEXOperandCode::G_V_E_xmm, /* 64bit */
                            )
                        } else {
                            (
                                Opcode::VFNMADD132SS,
                                VEXOperandCode::G_V_E_xmm, /* 64bit */
                            )
                        }
                    }
                    0x9E => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFNMSUB132PD,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        } else {
                            (
                                Opcode::VFNMSUB132PS,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        }
                    }
                    0x9F => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFNMSUB132SD,
                                VEXOperandCode::G_V_E_xmm, /* 64bit */
                            )
                        } else {
                            (
                                Opcode::VFNMSUB132SS,
                                VEXOperandCode::G_V_E_xmm, /* 64bit */
                            )
                        }
                    }
                    0xA6 => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFMADDSUB213PD,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        } else {
                            (
                                Opcode::VFMADDSUB213PS,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        }
                    }
                    0xA7 => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFMSUBADD213PD,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        } else {
                            (
                                Opcode::VFMSUBADD213PS,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        }
                    }
                    0xA8 => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFMADD213PD,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        } else {
                            (
                                Opcode::VFMADD213PS,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        }
                    }
                    0xA9 => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFMADD231SD,
                                VEXOperandCode::G_V_E_xmm, /* 64bit */
                            )
                        } else {
                            (
                                Opcode::VFMADD231SS,
                                VEXOperandCode::G_V_E_xmm, /* 64bit */
                            )
                        }
                    }
                    0xAA => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFMSUB213PD,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        } else {
                            (
                                Opcode::VFMSUB213PS,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        }
                    }
                    0xAB => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFMSUB231SD,
                                VEXOperandCode::G_V_E_xmm, /* 64bit */
                            )
                        } else {
                            (
                                Opcode::VFMSUB231SS,
                                VEXOperandCode::G_V_E_xmm, /* 64bit */
                            )
                        }
                    }
                    0xAC => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFNMADD213PD,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        } else {
                            (
                                Opcode::VFNMADD213PS,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        }
                    }
                    0xAD => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFNMADD213SD,
                                VEXOperandCode::G_V_E_xmm, /* 64bit */
                            )
                        } else {
                            (
                                Opcode::VFNMADD213SS,
                                VEXOperandCode::G_V_E_xmm, /* 64bit */
                            )
                        }
                    }
                    0xAE => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFNMSUB213PD,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        } else {
                            (
                                Opcode::VFNMSUB213PS,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        }
                    }
                    0xAF => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFNMSUB213SD,
                                VEXOperandCode::G_V_E_xmm, /* 64bit */
                            )
                        } else {
                            (
                                Opcode::VFNMSUB213SS,
                                VEXOperandCode::G_V_E_xmm, /* 64bit */
                            )
                        }
                    }
                    0xB6 => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFMADDSUB231PD,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        } else {
                            (
                                Opcode::VFMADDSUB231PS,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        }
                    }
                    0xB7 => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFMSUBADD231PD,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        } else {
                            (
                                Opcode::VFMSUBADD231PS,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        }
                    }
                    0xB8 => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFMADD231PD,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        } else {
                            (
                                Opcode::VFMADD231PS,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        }
                    }
                    0xB9 => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFMADD231SD,
                                VEXOperandCode::G_V_E_xmm, /* 64bit */
                            )
                        } else {
                            (
                                Opcode::VFMADD231SS,
                                VEXOperandCode::G_V_E_xmm, /* 64bit */
                            )
                        }
                    }
                    0xBA => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFMSUB231PD,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        } else {
                            (
                                Opcode::VFMSUB231PS,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        }
                    }
                    0xBB => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFMSUB231SD,
                                VEXOperandCode::G_V_E_xmm, /* 64bit */
                            )
                        } else {
                            (
                                Opcode::VFMSUB231SS,
                                VEXOperandCode::G_V_E_xmm, /* 64bit */
                            )
                        }
                    }
                    0xBC => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFNMADD231PD,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        } else {
                            (
                                Opcode::VFNMADD231PS,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        }
                    }
                    0xBD => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFNMADD231SD,
                                VEXOperandCode::G_V_E_xmm, /* 64bit */
                            )
                        } else {
                            (
                                Opcode::VFNMADD231SS,
                                VEXOperandCode::G_V_E_xmm, /* 64bit */
                            )
                        }
                    }
                    0xBE => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFNMSUB231PD,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        } else {
                            (
                                Opcode::VFNMSUB231PS,
                                if L {
                                    VEXOperandCode::G_V_E_ymm
                                } else {
                                    VEXOperandCode::G_V_E_ymm
                                },
                            )
                        }
                    }
                    0xBF => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VFNMSUB231SD,
                                VEXOperandCode::G_V_E_xmm, /* 64bit */
                            )
                        } else {
                            (
                                Opcode::VFNMSUB231SS,
                                VEXOperandCode::G_V_E_xmm, /* 64bit */
                            )
                        }
                    }
                    0xDB => (
                        Opcode::VAESIMC,
                        if L {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        } else {
                            VEXOperandCode::G_E_xmm
                        },
                    ),
                    0xDC => (
                        Opcode::VAESENC,
                        if L {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xDD => (
                        Opcode::VAESENCLAST,
                        if L {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xDE => (
                        Opcode::VAESDEC,
                        if L {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0xDF => (
                        Opcode::VAESDECLAST,
                        if L {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    _ => {
                        instruction.opcode = Opcode::Invalid;
                        return Err(DecodeError::InvalidOpcode);
                    }
                }
            } else {
                // the only VEX* 0f38 instructions have an implied 66 prefix.
                instruction.opcode = Opcode::Invalid;
                return Err(DecodeError::InvalidOpcode);
            }
        }
        VEXOpcodeMap::Map0F3A => {
            if let VEXOpcodePrefix::Prefix66 = p {
                // possibly valid!
                match opc {
                    0x00 => (
                        Opcode::VPERMQ,
                        if L {
                            VEXOperandCode::G_E_ymm_imm8
                        } else {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        },
                    ),
                    0x01 => (
                        Opcode::VPERMPD,
                        if L {
                            VEXOperandCode::G_E_ymm_imm8
                        } else {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        },
                    ),
                    0x02 => (
                        Opcode::VPBLENDD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x04 => (
                        Opcode::VPERMILPS,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x05 => (
                        Opcode::VPERMILPD,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            VEXOperandCode::G_V_E_xmm
                        },
                    ),
                    0x06 => (
                        Opcode::VPERM2F128,
                        if L {
                            VEXOperandCode::G_V_E_ymm
                        } else {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        },
                    ),
                    0x08 => (
                        Opcode::VROUNDPS,
                        if L {
                            VEXOperandCode::G_E_ymm_imm8
                        } else {
                            VEXOperandCode::G_E_xmm_imm8
                        },
                    ),
                    0x09 => (
                        Opcode::VROUNDPD,
                        if L {
                            VEXOperandCode::G_E_ymm_imm8
                        } else {
                            VEXOperandCode::G_E_xmm_imm8
                        },
                    ),
                    0x0C => (
                        Opcode::VBLENDPS,
                        if L {
                            VEXOperandCode::G_V_E_ymm_imm8
                        } else {
                            VEXOperandCode::G_V_E_xmm_imm8
                        },
                    ),
                    0x0D => (
                        Opcode::VBLENDPD,
                        if L {
                            VEXOperandCode::G_V_E_ymm_imm8
                        } else {
                            VEXOperandCode::G_V_E_xmm_imm8
                        },
                    ),
                    0x0E => (
                        Opcode::VPBLENDW,
                        if L {
                            VEXOperandCode::G_V_E_ymm_imm8
                        } else {
                            VEXOperandCode::G_V_E_xmm_imm8
                        },
                    ),
                    0x0F => (
                        Opcode::VPALIGNR,
                        if L {
                            VEXOperandCode::G_V_E_ymm_imm8
                        } else {
                            VEXOperandCode::G_V_E_xmm_imm8
                        },
                    ),
                    0x14 => (
                        Opcode::VPEXTRB,
                        if L || instruction.prefixes.vex().w() {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        } else {
                            VEXOperandCode::Ev_G_xmm_imm8
                        },
                    ),
                    0x15 => (
                        Opcode::VPEXTRW,
                        if L || instruction.prefixes.vex().w() {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        } else {
                            VEXOperandCode::Ev_G_xmm_imm8
                        },
                    ),
                    0x16 => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VPEXTRQ,
                                if L {
                                    instruction.opcode = Opcode::Invalid;
                                    return Err(DecodeError::InvalidOpcode);
                                } else {
                                    VEXOperandCode::G_E_ymm_imm8
                                },
                            )
                        } else {
                            (
                                Opcode::VPEXTRD,
                                if L {
                                    instruction.opcode = Opcode::Invalid;
                                    return Err(DecodeError::InvalidOpcode);
                                } else {
                                    // varies on W
                                    VEXOperandCode::Ev_G_xmm_imm8
                                },
                            )
                        }
                    }
                    0x17 => (
                        Opcode::VEXTRACTPS,
                        if L {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        } else {
                            VEXOperandCode::G_E_ymm_imm8
                        },
                    ),
                    0x18 => (
                        Opcode::VINSERTF128,
                        if L {
                            VEXOperandCode::G_V_E_ymm_imm8
                        } else {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        },
                    ),
                    0x19 => (
                        Opcode::VEXTRACTF128,
                        if L {
                            VEXOperandCode::E_xmm_G_ymm_imm8
                        } else {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        },
                    ),
                    0x1D => (
                        Opcode::VCVTPS2PH,
                        if L {
                            VEXOperandCode::E_xmm_G_ymm_imm8
                        } else {
                            VEXOperandCode::E_G_xmm_imm8
                        },
                    ),
                    0x20 => (
                        Opcode::VPINSRB,
                        if L {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        } else {
                            VEXOperandCode::G_V_E_xmm_imm8
                        },
                    ),
                    0x21 => (
                        Opcode::VINSERTPS,
                        if L {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        } else {
                            VEXOperandCode::G_V_E_xmm_imm8
                        },
                    ),
                    0x22 => {
                        if instruction.prefixes.vex().w() {
                            (
                                Opcode::VPINSRQ,
                                if L {
                                    instruction.opcode = Opcode::Invalid;
                                    return Err(DecodeError::InvalidOpcode);
                                } else {
                                    VEXOperandCode::G_V_E_xmm_imm8
                                },
                            )
                        } else {
                            (
                                Opcode::VPINSRD,
                                if L {
                                    instruction.opcode = Opcode::Invalid;
                                    return Err(DecodeError::InvalidOpcode);
                                } else {
                                    VEXOperandCode::G_V_E_xmm_imm8
                                },
                            )
                        }
                    }
                    0x38 => (
                        Opcode::VINSERTI128,
                        if L {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        } else {
                            VEXOperandCode::V_ymm_G_ymm_E_xmm_imm8
                        },
                    ),
                    0x39 => (
                        Opcode::VEXTRACTI128,
                        if L {
                            VEXOperandCode::V_xmm_G_ymm_E_ymm_imm8
                        } else {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        },
                    ),
                    0x40 => (
                        Opcode::VDPPS,
                        if L {
                            VEXOperandCode::G_V_E_ymm_imm8
                        } else {
                            VEXOperandCode::G_V_E_xmm_imm8
                        },
                    ),
                    0x41 => (
                        Opcode::VDPPD,
                        if L {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        } else {
                            VEXOperandCode::G_V_E_xmm_imm8
                        },
                    ),
                    0x42 => (
                        Opcode::VMPSADBW,
                        if L {
                            VEXOperandCode::G_E_ymm_imm8
                        } else {
                            VEXOperandCode::G_E_xmm_imm8
                        },
                    ),
                    0x44 => (
                        Opcode::VPCLMULQDQ,
                        if L {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        } else {
                            VEXOperandCode::G_V_E_xmm_imm8
                        },
                    ),
                    0x46 => (
                        Opcode::VPERM2I128,
                        if L {
                            VEXOperandCode::G_V_E_ymm_imm8
                        } else {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        },
                    ),
                    0x4A => (
                        Opcode::VBLENDVPS,
                        if L {
                            VEXOperandCode::G_V_E_ymm_ymm4
                        } else {
                            VEXOperandCode::G_V_E_xmm_xmm4
                        },
                    ),
                    0x4B => (
                        Opcode::VBLENDVPD,
                        if L {
                            VEXOperandCode::G_V_E_ymm_ymm4
                        } else {
                            VEXOperandCode::G_V_E_xmm_xmm4
                        },
                    ),
                    0x4C => (
                        Opcode::VPBLENDVB,
                        if L {
                            VEXOperandCode::G_V_E_ymm_ymm4
                        } else {
                            VEXOperandCode::G_V_E_xmm_xmm4
                        },
                    ),
                    0x62 => (
                        Opcode::VPCMPISTRM,
                        if L {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        } else {
                            VEXOperandCode::G_E_xmm_imm8
                        },
                    ),
                    0x63 => (
                        Opcode::VPCMPISTRI,
                        if L {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        } else {
                            VEXOperandCode::G_E_xmm_imm8
                        },
                    ),
                    0xDF => (
                        Opcode::VAESKEYGENASSIST,
                        if L {
                            instruction.opcode = Opcode::Invalid;
                            return Err(DecodeError::InvalidOpcode);
                        } else {
                            VEXOperandCode::G_E_xmm_imm8
                        },
                    ),
                    _ => {
                        instruction.opcode = Opcode::Invalid;
                        return Err(DecodeError::InvalidOpcode);
                    }
                }
            } else {
                // the only VEX* 0f3a instructions have an implied 66 prefix.
                instruction.opcode = Opcode::Invalid;
                return Err(DecodeError::InvalidOpcode);
            }
        }
    };
    instruction.opcode = opcode;
    read_vex_operands(bytes, instruction, length, operand_code)
}
