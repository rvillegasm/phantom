use phantom::nes::cpu::Cpu;
use phantom::nes::memory::Memory;
use phantom::nes::opcodes::{AddressingMode, OpCode, OPCODES_MAP};
use std::collections::HashMap;

pub fn trace(cpu: &Cpu) -> String {
    let ref opcodes: HashMap<u8, &'static OpCode> = *OPCODES_MAP;

    let code = cpu.mem_read(cpu.program_counter());
    let op = opcodes.get(&code).unwrap();

    let start = cpu.program_counter();
    let mut hex_dump = vec![];
    hex_dump.push(code);

    let (mem_addr, stored_value) = match op.mode() {
        AddressingMode::Immediate | AddressingMode::NoneAddressing => (0, 0),
        _ => {
            let addr = cpu.get_real_address(&op.mode(), start + 1);
            (addr, cpu.mem_read(addr))
        }
    };

    let tmp = match op.len() {
        1 => match op.code() {
            0x0A | 0x4A | 0x2A | 0x6A => format!("A "),
            _ => String::from(""),
        },
        2 => {
            let addr = cpu.mem_read(start + 1);
            hex_dump.push(addr);

            match op.mode() {
                AddressingMode::Immediate => format!("#${:02x}", addr),
                AddressingMode::ZeroPage => format!("${:02x} = {:02x}", mem_addr, stored_value),
                AddressingMode::ZeroPageX => {
                    format!("${:02x},X @ {:02x} = {:02x}", addr, mem_addr, stored_value)
                }
                AddressingMode::ZeroPageY => {
                    format!("${:02x},Y @ {:02x} = {:02x}", addr, mem_addr, stored_value)
                }
                AddressingMode::IndirectX => format!(
                    "(${:02x},X) @ {:02x} = {:04x} = {:02x}",
                    addr,
                    (addr.wrapping_add(cpu.register_x())),
                    mem_addr,
                    stored_value
                ),
                AddressingMode::IndirectY => format!(
                    "(${:02x}),Y = {:04x} @ {:04x} = {:02x}",
                    addr,
                    (mem_addr.wrapping_sub(cpu.register_y() as u16)),
                    mem_addr,
                    stored_value
                ),
                AddressingMode::NoneAddressing => {
                    // assuming local jumps: BNE, BVS, etc....
                    let addr: usize = (start as usize + 2).wrapping_add((addr as i8) as usize);
                    format!("${:04x}", addr)
                }
                _ => panic!(
                    "Unexpected addressing mode {:?} has ops-len 2. Code {:02x}",
                    op.mode(),
                    op.code()
                ),
            }
        }
        3 => {
            let addr_lo = cpu.mem_read(start + 1);
            let addr_hi = cpu.mem_read(start + 2);
            hex_dump.push(addr_lo);
            hex_dump.push(addr_hi);

            let addr = cpu.mem_read_u16(start + 1);

            match op.mode() {
                AddressingMode::NoneAddressing => {
                    if op.code() == 0x6C {
                        // JMP indirect
                        let jmp_addr = if addr & 0x00FF == 0x00FF {
                            let lo = cpu.mem_read(addr);
                            let hi = cpu.mem_read(addr & 0xFF00);
                            (hi as u16) << 8 | (lo as u16)
                        } else {
                            cpu.mem_read_u16(addr)
                        };

                        format!("(${:04x}) = {:04x}", addr, jmp_addr)
                    } else {
                        format!("${:04x}", addr)
                    }
                }
                AddressingMode::Absolute => format!("${:04x} = {:02x}", mem_addr, stored_value),
                AddressingMode::AbsoluteX => {
                    format!("${:04x},X @ {:04x} = {:02x}", addr, mem_addr, stored_value)
                }
                AddressingMode::AbsoluteY => {
                    format!("${:04x},Y @ {:04x} = {:02x}", addr, mem_addr, stored_value)
                }
                _ => panic!(
                    "Unexpected addressing mode {:?} has ops-len 3. Code {:02x}",
                    op.mode(),
                    op.code()
                ),
            }
        }
        _ => String::from(""),
    };

    let hex_str = hex_dump
        .iter()
        .map(|z| format!("{:02x}", z))
        .collect::<Vec<String>>()
        .join(" ");

    let asm_str = format!("{:04x}  {:8} {: >4} {}", start, hex_str, op.mnemonic(), tmp)
        .trim()
        .to_string();

    format!(
        "{:47} A:{:02x} X:{:02x} Y:{:02x} P:{:02x} SP:{:02x}",
        asm_str,
        cpu.register_a(),
        cpu.register_x(),
        cpu.register_y(),
        cpu.status(),
        cpu.stack_pointer()
    )
    .to_ascii_uppercase()
}
