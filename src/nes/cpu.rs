use crate::nes::bus::Bus;
/// Implementation of the NES' custom 6502 CPU
use crate::nes::memory::Memory;
use crate::nes::opcodes::{AddressingMode, OpCode, OPCODES_MAP};
use crate::nes::interrupt;
use bitflags::bitflags;
use std::collections::HashMap;

const ZEROTH_BIT: u8 = 0b00000001;
const FIRST_BIT: u8 = 0b00000010;
const SECOND_BIT: u8 = 0b00000100;
const THIRD_BIT: u8 = 0b00001000;
const FOURTH_BIT: u8 = 0b00010000;
const FIFTH_BIT: u8 = 0b00100000;
const SIXTH_BIT: u8 = 0b01000000;
const SEVENTH_BIT: u8 = 0b10000000;

const PROGRAM_ROM_START_ADDR: u16 = 0x8000;

const STACK_START_ADDR: u16 = 0x0100;
const STACK_RESET_ADDR: u8 = 0xFD;

bitflags! {
    /// # Status Register (P) http://wiki.nesdev.com/w/index.php/Status_flags
    ///
    ///  7 6 5 4 3 2 1 0
    ///  N V _ B D I Z C
    ///  | |   | | | | +--- Carry Flag
    ///  | |   | | | +----- Zero Flag
    ///  | |   | | +------- Interrupt Disable
    ///  | |   | +--------- Decimal Mode (not used on NES)
    ///  | |   +----------- Break Command
    ///  | +--------------- Overflow Flag
    ///  +----------------- Negative Flag
    ///
    pub struct CpuFlags: u8 {
        const CARRY             = ZEROTH_BIT;
        const ZERO              = FIRST_BIT;
        const INTERRUPT_DISABLE = SECOND_BIT;
        const DECIMAL_MODE      = THIRD_BIT;
        const BREAK             = FOURTH_BIT;
        const BREAK2            = FIFTH_BIT;
        const OVERFLOW          = SIXTH_BIT;
        const NEGATIVE          = SEVENTH_BIT;
    }
}

pub struct Cpu<'a> {
    register_a: u8,
    register_x: u8,
    register_y: u8,
    status: CpuFlags,
    program_counter: u16,
    stack_pointer: u8,
    bus: Bus<'a>,
}

impl Memory for Cpu<'_> {
    fn mem_read(&mut self, addr: u16) -> u8 {
        self.bus.mem_read(addr)
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        self.bus.mem_write(addr, data);
    }

    fn mem_read_u16(&mut self, addr: u16) -> u16 {
        self.bus.mem_read_u16(addr)
    }

    fn mem_write_u16(&mut self, addr: u16, data: u16) {
        self.bus.mem_write_u16(addr, data);
    }
}

impl<'a> Cpu<'a> {
    pub fn new<'b>(bus: Bus<'b>) -> Cpu<'b> {
        Cpu {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            status: CpuFlags::from_bits_truncate(0b100100),
            program_counter: 0,
            stack_pointer: STACK_RESET_ADDR,
            bus,
        }
    }

    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.status = CpuFlags::from_bits_truncate(0b100100);
        self.stack_pointer = STACK_RESET_ADDR;

        self.program_counter = self.mem_read_u16(0xFFFC);
    }

    #[deprecated = "No longer usable due to prg_rom being looked for writes"]
    pub fn load_and_run(&mut self, program: Vec<u8>) {
        self.load(program);
        self.reset();
        self.run();
    }

    #[deprecated = "No longer usable due to prg_rom being looked for writes"]
    pub fn load(&mut self, program: Vec<u8>) {
        for i in 0..(program.len() as u16) {
            self.mem_write(PROGRAM_ROM_START_ADDR + i, program[i as usize]);
        }
        self.mem_write_u16(0xFFFC, PROGRAM_ROM_START_ADDR);
    }

    pub fn run(&mut self) {
        self.run_with_callback(|_| {});
    }

    pub fn run_with_callback<F>(&mut self, mut callback: F)
    where
        F: FnMut(&mut Cpu),
    {
        let ref opcodes: HashMap<u8, &'static OpCode> = *OPCODES_MAP;

        loop {
            if let Some(nmi) = self.bus.poll_nmi_status() {
                self.manage_interrupt(interrupt::NMI);
            }

            callback(self);

            let code = self.mem_read(self.program_counter);
            self.program_counter += 1;
            let program_counter_state = self.program_counter;

            let opcode = opcodes
                .get(&code)
                .expect(&format!("OpCode {:x} could not be recognised!", code));

            match code {
                0xEA => { /* NOP - Do Nothing */ }
                0x00 => return,
                0x40 => {
                    self.rti();
                }
                0x69 | 0x65 | 0x75 | 0x6D | 0x7D | 0x79 | 0x61 | 0x71 => self.adc(opcode.mode()),
                0x29 | 0x25 | 0x35 | 0x2D | 0x3D | 0x39 | 0x21 | 0x31 => {
                    self.and(opcode.mode());
                }
                0x0A => {
                    self.asl_accumulator();
                }
                0x06 | 0x16 | 0x0E | 0x1E => {
                    self.asl(opcode.mode());
                }
                0x24 | 0x2c => {
                    self.bit(opcode.mode());
                }
                0xC9 | 0xC5 | 0xD5 | 0xCD | 0xDD | 0xD9 | 0xC1 | 0xD1 => {
                    self.compare(opcode.mode(), self.register_a); // CMP
                }
                0xC6 | 0xD6 | 0xCE | 0xDE => {
                    self.dec(opcode.mode());
                }
                0x49 | 0x45 | 0x55 | 0x4D | 0x5D | 0x59 | 0x41 | 0x51 => {
                    self.eor(opcode.mode());
                }
                0x4A => {
                    self.lsr_accumulator();
                }
                0x46 | 0x56 | 0x4E | 0x5E => {
                    self.lsr(opcode.mode());
                }
                0x09 | 0x05 | 0x15 | 0x0D | 0x1D | 0x19 | 0x01 | 0x11 => {
                    self.ora(opcode.mode());
                }
                0x2A => {
                    self.rol_accumulator();
                }
                0x26 | 0x36 | 0x2E | 0x3E => {
                    self.rol(opcode.mode());
                }
                0x6A => {
                    self.ror_accumulator();
                }
                0x66 | 0x76 | 0x6E | 0x7E => {
                    self.ror(opcode.mode());
                }
                0xE9 | 0xE5 | 0xF5 | 0xED | 0xFD | 0xF9 | 0xE1 | 0xF1 => {
                    self.sbc(opcode.mode());
                }
                0x90 => {
                    self.branch(!self.status.contains(CpuFlags::CARRY)); // BCC
                }
                0xB0 => {
                    self.branch(self.status.contains(CpuFlags::CARRY)); // BCS
                }
                0xF0 => {
                    self.branch(self.status.contains(CpuFlags::ZERO)); // BEQ
                }
                0x30 => {
                    self.branch(self.status.contains(CpuFlags::NEGATIVE)); // BMI
                }
                0xD0 => {
                    self.branch(!self.status.contains(CpuFlags::ZERO)); // BNE
                }
                0x10 => {
                    self.branch(!self.status.contains(CpuFlags::NEGATIVE)); // BPL
                }
                0x50 => {
                    self.branch(!self.status.contains(CpuFlags::OVERFLOW)); // BVC
                }
                0x70 => {
                    self.branch(self.status.contains(CpuFlags::OVERFLOW)); // BVS
                }
                0x4C => {
                    self.jmp_absolute();
                }
                0x6C => {
                    self.jmp_indirect();
                }
                0x20 => {
                    self.jsr();
                }
                0x60 => {
                    self.rts();
                }
                0xE0 | 0xE4 | 0xEC => {
                    self.compare(opcode.mode(), self.register_x); // CPX
                }
                0xC0 | 0xC4 | 0xCC => {
                    self.compare(opcode.mode(), self.register_y); // CPY
                }
                0xCA => {
                    self.dex();
                }
                0x88 => {
                    self.dey();
                }
                0xE6 | 0xF6 | 0xEE | 0xFE => {
                    self.inc(opcode.mode());
                }
                0xE8 => {
                    self.inx();
                }
                0xC8 => {
                    self.iny();
                }
                0xA9 | 0xA5 | 0xB5 | 0xAD | 0xBD | 0xB9 | 0xA1 | 0xB1 => {
                    self.lda(opcode.mode());
                }
                0xA2 | 0xA6 | 0xB6 | 0xAE | 0xBE => {
                    self.ldx(opcode.mode());
                }
                0xA0 | 0xA4 | 0xB4 | 0xAC | 0xBC => {
                    self.ldy(opcode.mode());
                }
                0x85 | 0x95 | 0x8D | 0x9D | 0x99 | 0x81 | 0x91 => {
                    self.sta(opcode.mode());
                }
                0x86 | 0x96 | 0x8E => {
                    self.stx(opcode.mode());
                }
                0x84 | 0x94 | 0x8C => {
                    self.sty(opcode.mode());
                }
                0xAA => {
                    self.tax();
                }
                0xA8 => {
                    self.tay();
                }
                0xBA => {
                    self.tsx();
                }
                0x8A => {
                    self.txa();
                }
                0x9A => {
                    self.txs();
                }
                0x98 => {
                    self.tya();
                }
                0x18 => {
                    self.clear_carry_flag(); // CLC
                }
                0xD8 => {
                    self.clear_decimal_mode_flag(); // CLD
                }
                0x58 => {
                    self.clear_interrupt_disable_flag(); // CLI
                }
                0xB8 => {
                    self.clear_overflow_flag(); // CLV
                }
                0x38 => {
                    self.set_carry_flag(); // SEC
                }
                0xF8 => {
                    self.set_decimal_mode_flag(); // SED
                }
                0x78 => {
                    self.set_interrupt_disable_flag(); // SEI
                }
                0x48 => {
                    self.pha();
                }
                0x08 => {
                    self.php();
                }
                0x68 => {
                    self.pla();
                }
                0x28 => {
                    self.plp();
                }
                0xC7 | 0xD7 | 0xCF | 0xDF | 0xDB | 0xD3 | 0xC3 => {
                    self.dcp(opcode.mode());
                }
                0x27 | 0x37 | 0x2F | 0x3F | 0x3B | 0x33 | 0x23 => {
                    self.rla(opcode.mode());
                }
                0x07 | 0x17 | 0x0F | 0x1F | 0x1B | 0x03 | 0x13 => {
                    self.slo(opcode.mode());
                }
                0x47 | 0x57 | 0x4F | 0x5F | 0x5B | 0x43 | 0x53 => {
                    self.sre(opcode.mode());
                }
                0x80 | 0x82 | 0x89 | 0xc2 | 0xe2 => {
                    // SKB (NOP immediate)
                }
                0xCB => {
                    self.axs(opcode.mode());
                }
                0x6B => {
                    self.arr(opcode.mode());
                }
                0xEB => {
                    self.sbc_unofficial(opcode.mode());
                }
                0x0B | 0x2B => {
                    self.anc(opcode.mode());
                }
                0x4B => {
                    self.alr(opcode.mode());
                }
                0x04 | 0x44 | 0x64 | 0x14 | 0x34 | 0x54 | 0x74 | 0xD4 | 0xF4 | 0x0C | 0x1C
                | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC => {
                    // NOP read
                    let (addr, page_cross) = self.compute_operand_address(opcode.mode());
                    let _mem_value = self.mem_read(addr);
                    if page_cross {
                        self.bus.tick(1);
                    }
                }
                0x67 | 0x77 | 0x6F | 0x7F | 0x7B | 0x63 | 0x73 => {
                    self.rra(opcode.mode());
                }
                0xE7 | 0xF7 | 0xEF | 0xFF | 0xFB | 0xE3 | 0xF3 => {
                    self.isb(opcode.mode());
                }
                0x02 | 0x12 | 0x22 | 0x32 | 0x42 | 0x52 | 0x62 | 0x72 | 0x92 | 0xB2 | 0xD2
                | 0xF2 => {
                    // NOP - do nothing
                }
                0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xFA => {
                    // NOP - do nothing
                }
                0xAB => {
                    self.lxa(opcode.mode());
                }
                0x8B => {
                    self.xaa(opcode.mode());
                }
                0xBB => {
                    self.las(opcode.mode());
                }
                0x9B => {
                    self.tas(opcode.mode());
                }
                0x93 | 0x9F => {
                    self.ahx(opcode.mode());
                }
                0x9E => {
                    self.shx(opcode.mode());
                }
                0x9C => {
                    self.shy(opcode.mode());
                }
                0xA7 | 0xB7 | 0xAF | 0xBF | 0xA3 | 0xB3 => {
                    self.lax(opcode.mode());
                }
                0x87 | 0x97 | 0x8F | 0x83 => {
                    self.sax(opcode.mode());
                }
                _ => {
                    panic!("OpCode {} is not a valid instruction!", code);
                }
            }

            self.bus.tick(opcode.cycles());

            if program_counter_state == self.program_counter {
                self.program_counter += (opcode.len() - 1) as u16;
            }
        }
    }

    fn rti(&mut self) {
        self.status.bits = self.stack_pop();
        self.status.remove(CpuFlags::BREAK);
        self.status.insert(CpuFlags::BREAK2);

        self.program_counter = self.stack_pop_u16();
    }

    fn adc(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.compute_operand_address(mode);
        let mem_value = self.mem_read(addr);
        self.add_to_register_a(mem_value);

        if page_cross {
            self.bus.tick(1);
        }
    }

    fn and(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.compute_operand_address(mode);
        let mem_value = self.mem_read(addr);
        self.set_register_a(mem_value & self.register_a);

        if page_cross {
            self.bus.tick(1);
        }
    }

    fn asl(&mut self, mode: &AddressingMode) -> u8 {
        let (addr, _) = self.compute_operand_address(mode);
        let mut mem_value = self.mem_read(addr);

        let carry_val = SEVENTH_BIT & mem_value;
        if carry_val == SEVENTH_BIT {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }

        mem_value = mem_value << 1;
        self.mem_write(addr, mem_value);
        self.update_zero_flag(mem_value);
        self.update_negative_flag(mem_value);
        mem_value
    }

    fn asl_accumulator(&mut self) {
        let carry_val = SEVENTH_BIT & self.register_a;
        if carry_val == SEVENTH_BIT {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }
        self.set_register_a(self.register_a << 1);
    }

    fn bit(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.compute_operand_address(mode);
        let mem_value = self.mem_read(addr);
        let and_val = mem_value & self.register_a;

        if and_val == 0 {
            self.status.insert(CpuFlags::ZERO);
        } else {
            self.status.remove(CpuFlags::ZERO);
        }

        self.status
            .set(CpuFlags::NEGATIVE, mem_value & SEVENTH_BIT > 0);
        self.status
            .set(CpuFlags::OVERFLOW, mem_value & SIXTH_BIT > 0);
    }

    fn compare(&mut self, mode: &AddressingMode, val_to_compare: u8) {
        let (addr, page_cross) = self.compute_operand_address(mode);
        let mem_value = self.mem_read(addr);

        if val_to_compare >= mem_value {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }

        let subtracted_val = val_to_compare.wrapping_sub(mem_value);
        self.update_zero_flag(subtracted_val);
        self.update_negative_flag(subtracted_val);

        if page_cross {
            self.bus.tick(1);
        }
    }

    fn dec(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.compute_operand_address(mode);
        let mut mem_value = self.mem_read(addr);
        mem_value = mem_value.wrapping_sub(1);
        self.mem_write(addr, mem_value);
        self.update_zero_flag(mem_value);
        self.update_negative_flag(mem_value);
    }

    fn eor(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.compute_operand_address(mode);
        let mem_value = self.mem_read(addr);
        self.set_register_a(self.register_a ^ mem_value);

        if page_cross {
            self.bus.tick(1);
        }
    }

    fn lsr(&mut self, mode: &AddressingMode) -> u8 {
        let (addr, _) = self.compute_operand_address(mode);
        let mut mem_value = self.mem_read(addr);

        if mem_value & ZEROTH_BIT == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }

        mem_value = mem_value >> 1;
        self.mem_write(addr, mem_value);
        self.update_zero_flag(mem_value);
        self.update_negative_flag(mem_value);
        mem_value
    }

    fn lsr_accumulator(&mut self) {
        let carry_val = self.register_a & ZEROTH_BIT;
        if carry_val == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }
        self.set_register_a(self.register_a >> 1);
    }

    fn ora(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.compute_operand_address(mode);
        let mem_value = self.mem_read(addr);
        self.set_register_a(self.register_a | mem_value);

        if page_cross {
            self.bus.tick(1);
        }
    }

    fn rol(&mut self, mode: &AddressingMode) -> u8 {
        let (addr, _) = self.compute_operand_address(mode);
        let mem_value = self.mem_read(addr);
        let result = self.rol_internal(mem_value);
        self.mem_write(addr, result);
        self.update_zero_flag(result);
        self.update_negative_flag(result);
        result
    }

    fn rol_accumulator(&mut self) {
        let result = self.rol_internal(self.register_a);
        self.set_register_a(result);
    }

    fn rol_internal(&mut self, data: u8) -> u8 {
        let old_carry_val = self.status.contains(CpuFlags::CARRY);

        if data >> 7 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }

        let mut result = data << 1;
        if old_carry_val {
            result |= 1;
        }
        result
    }

    fn ror(&mut self, mode: &AddressingMode) -> u8 {
        let (addr, _) = self.compute_operand_address(mode);
        let mem_value = self.mem_read(addr);
        let result = self.ror_internal(mem_value);
        self.mem_write(addr, result);
        self.update_zero_flag(result);
        self.update_negative_flag(result);
        result
    }

    fn ror_accumulator(&mut self) {
        let result = self.ror_internal(self.register_a);
        self.set_register_a(result);
    }

    fn ror_internal(&mut self, data: u8) -> u8 {
        let old_carry_val = self.status.contains(CpuFlags::CARRY);

        if data & ZEROTH_BIT == ZEROTH_BIT {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }

        let mut result = data >> 1;
        if old_carry_val {
            result |= SEVENTH_BIT;
        }
        result
    }

    fn sbc(&mut self, mode: &AddressingMode) {
        // http://www.6502.org/tutorials/6502opcodes.html#SBC
        // To subtract you set the carry before the operation.
        // If the carry is cleared by the operation, it indicates a borrow occurred.
        let (addr, page_cross) = self.compute_operand_address(mode);
        let mem_value = self.mem_read(addr);
        self.sub_to_register_a(mem_value);

        if page_cross {
            self.bus.tick(1);
        }
    }

    fn branch(&mut self, condition: bool) {
        if condition {
            self.bus.tick(1);

            let jump_amount = self.mem_read(self.program_counter) as i8;
            let jump_addr = self
                .program_counter
                .wrapping_add(1)
                .wrapping_add(jump_amount as u16); // Relative jumping

            if self.page_cross(self.program_counter.wrapping_add(1), jump_addr) {
                self.bus.tick(1);
            }

            self.program_counter = jump_addr;
        }
    }

    fn jmp_absolute(&mut self) {
        let addr = self.mem_read_u16(self.program_counter);
        self.program_counter = addr;
    }

    fn jmp_indirect(&mut self) {
        let addr = self.mem_read_u16(self.program_counter);
        // The original 6502 has a bug that does not correctly fetch the target address
        // if the indirect vector falls on a page boundary
        // (e.g. $xxFF where xx is any value from $00 to $FF).
        // In such a case, it fetches the LSB from $xxFF as expected but takes the MSB from $xx00.
        let indirect_addr = if addr & 0x00FF == 0x00FF {
            let lo = self.mem_read(addr);
            let hi = self.mem_read(addr & 0xFF00);
            (hi as u16) << 8 | (lo as u16)
        } else {
            self.mem_read_u16(addr)
        };

        self.program_counter = indirect_addr;
    }

    fn jsr(&mut self) {
        self.stack_push_u16(self.program_counter + 2 /*+2 extra bytes to read*/ - 1);
        let addr = self.mem_read_u16(self.program_counter);
        self.program_counter = addr;
    }

    fn rts(&mut self) {
        self.program_counter = self.stack_pop_u16() + 1;
    }

    fn dex(&mut self) {
        self.set_register_x(self.register_x.wrapping_sub(1));
    }

    fn dey(&mut self) {
        self.set_register_y(self.register_y.wrapping_sub(1));
    }

    fn inc(&mut self, mode: &AddressingMode) -> u8 {
        let (addr, _) = self.compute_operand_address(mode);
        let mut mem_value = self.mem_read(addr);
        mem_value = mem_value.wrapping_add(1);
        self.mem_write(addr, mem_value);
        self.update_zero_flag(mem_value);
        self.update_negative_flag(mem_value);
        mem_value
    }

    fn inx(&mut self) {
        self.set_register_x(self.register_x.wrapping_add(1));
    }

    fn iny(&mut self) {
        self.set_register_y(self.register_y.wrapping_add(1));
    }

    fn lda(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.compute_operand_address(mode);
        let mem_value = self.mem_read(addr);
        self.set_register_a(mem_value);

        if page_cross {
            self.bus.tick(1);
        }
    }

    fn ldx(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.compute_operand_address(mode);
        let mem_value = self.mem_read(addr);
        self.set_register_x(mem_value);

        if page_cross {
            self.bus.tick(1);
        }
    }

    fn ldy(&mut self, mode: &AddressingMode) {
        let (addr, page_cross) = self.compute_operand_address(mode);
        let mem_value = self.mem_read(addr);
        self.set_register_y(mem_value);

        if page_cross {
            self.bus.tick(1);
        }
    }

    fn sta(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.compute_operand_address(mode);
        self.mem_write(addr, self.register_a);
    }

    fn stx(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.compute_operand_address(mode);
        self.mem_write(addr, self.register_x);
    }

    fn sty(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.compute_operand_address(mode);
        self.mem_write(addr, self.register_y);
    }

    fn tax(&mut self) {
        self.set_register_x(self.register_a);
    }

    fn tay(&mut self) {
        self.set_register_y(self.register_a);
    }

    fn tsx(&mut self) {
        self.set_register_x(self.stack_pointer);
    }

    fn txa(&mut self) {
        self.set_register_a(self.register_x);
    }

    fn txs(&mut self) {
        self.stack_pointer = self.register_x;
    }

    fn tya(&mut self) {
        self.set_register_a(self.register_y);
    }

    fn pha(&mut self) {
        self.stack_push(self.register_a);
    }

    fn php(&mut self) {
        // http://wiki.nesdev.com/w/index.php/CPU_status_flag_behavior
        let mut status_flags = self.status.clone();
        status_flags.insert(CpuFlags::BREAK);
        status_flags.insert(CpuFlags::BREAK2);
        self.stack_push(status_flags.bits());
    }

    fn pla(&mut self) {
        let data = self.stack_pop();
        self.set_register_a(data);
    }

    fn plp(&mut self) {
        self.status.bits = self.stack_pop();
        self.status.remove(CpuFlags::BREAK);
        self.status.insert(CpuFlags::BREAK2);
    }

    fn dcp(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.compute_operand_address(mode);
        let mut mem_value = self.mem_read(addr);
        mem_value = mem_value.wrapping_sub(1);
        self.mem_write(addr, mem_value);

        if mem_value <= self.register_a {
            self.set_carry_flag();
        }
        self.update_zero_flag(self.register_a.wrapping_sub(mem_value));
        self.update_negative_flag(self.register_a.wrapping_sub(mem_value));
    }

    fn rla(&mut self, mode: &AddressingMode) {
        let rotated_left_val = self.rol(mode);
        self.and_with_register_a(rotated_left_val);
    }

    fn slo(&mut self, mode: &AddressingMode) {
        let left_shifted_val = self.asl(mode);
        self.or_with_register_a(left_shifted_val);
    }

    fn sre(&mut self, mode: &AddressingMode) {
        let right_shifted_val = self.lsr(mode);
        self.xor_with_register_a(right_shifted_val);
    }

    fn axs(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.compute_operand_address(mode);
        let mem_value = self.mem_read(addr);
        let x_and_a_val = self.register_x & self.register_a;
        let result = x_and_a_val.wrapping_sub(mem_value);

        if mem_value <= x_and_a_val {
            self.set_carry_flag();
        }

        self.set_register_x(result);
    }

    fn arr(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.compute_operand_address(mode);
        let mem_value = self.mem_read(addr);

        self.and_with_register_a(mem_value);
        self.ror_accumulator();

        let bit_5 = (self.register_a >> 5) & 1;
        let bit_6 = (self.register_a >> 6) & 1;

        if bit_6 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }

        if bit_5 ^ bit_6 == 1 {
            self.set_overflow_flag();
        } else {
            self.clear_overflow_flag();
        }
    }

    fn sbc_unofficial(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.compute_operand_address(mode);
        let mem_value = self.mem_read(addr);
        self.sub_to_register_a(mem_value);
    }

    fn anc(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.compute_operand_address(mode);
        let mem_value = self.mem_read(addr);
        self.and_with_register_a(mem_value);

        if self.status.contains(CpuFlags::NEGATIVE) {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }
    }

    fn alr(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.compute_operand_address(mode);
        let mem_value = self.mem_read(addr);

        self.and_with_register_a(mem_value);
        self.lsr_accumulator();
    }

    fn rra(&mut self, mode: &AddressingMode) {
        let right_shifted_val = self.ror(mode);
        self.add_to_register_a(right_shifted_val);
    }

    fn isb(&mut self, mode: &AddressingMode) {
        let inc_val = self.inc(mode);
        self.sub_to_register_a(inc_val);
    }

    fn lxa(&mut self, mode: &AddressingMode) {
        self.lda(mode);
        self.tax();
    }

    fn xaa(&mut self, mode: &AddressingMode) {
        self.set_register_a(self.register_x);
        let (addr, _) = self.compute_operand_address(mode);
        let mem_value = self.mem_read(addr);
        self.and_with_register_a(mem_value);
    }

    fn las(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.compute_operand_address(mode);
        let mut mem_value = self.mem_read(addr);
        mem_value &= self.stack_pointer;
        self.register_a = mem_value; // Code repetition to avoid unnecessary multiple flag updates
        self.register_x = mem_value;
        self.stack_pointer = mem_value;
        self.update_zero_flag(mem_value);
        self.update_negative_flag(mem_value);
    }

    fn tas(&mut self, mode: &AddressingMode) {
        self.stack_pointer = self.register_a & self.register_x;
        let (addr, _) = self.compute_operand_address(mode);
        let result = ((addr >> 8) as u8 + 1) & self.stack_pointer;
        self.mem_write(addr, result);
    }

    fn ahx(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.compute_operand_address(mode);
        let result = self.register_a & self.register_x & (addr >> 8) as u8;
        self.mem_write(addr, result);
    }

    fn shx(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.compute_operand_address(mode);
        let result = self.register_x & ((addr >> 8) as u8 + 1);
        self.mem_write(addr, result);
    }

    fn shy(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.compute_operand_address(mode);
        let result = self.register_y & ((addr >> 8) as u8 + 1);
        self.mem_write(addr, result);
    }

    fn lax(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.compute_operand_address(mode);
        let mem_value = self.mem_read(addr);
        self.set_register_a(mem_value);
        self.register_x = mem_value;
    }

    fn sax(&mut self, mode: &AddressingMode) {
        let (addr, _) = self.compute_operand_address(mode);
        self.mem_write(addr, self.register_a & self.register_x);
    }

    fn stack_push(&mut self, data: u8) {
        self.mem_write(STACK_START_ADDR + (self.stack_pointer as u16), data);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
    }

    fn stack_pop(&mut self) -> u8 {
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        self.mem_read(STACK_START_ADDR + (self.stack_pointer as u16))
    }

    fn stack_push_u16(&mut self, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xFF) as u8;
        self.stack_push(hi);
        self.stack_push(lo);
    }

    fn stack_pop_u16(&mut self) -> u16 {
        let lo = self.stack_pop() as u16;
        let hi = self.stack_pop() as u16;
        hi << 8 | lo
    }

    fn update_zero_flag(&mut self, result: u8) {
        if result == 0 {
            self.status.insert(CpuFlags::ZERO);
        } else {
            self.status.remove(CpuFlags::ZERO);
        }
    }

    fn update_negative_flag(&mut self, result: u8) {
        if result & SEVENTH_BIT != 0 {
            self.status.insert(CpuFlags::NEGATIVE);
        } else {
            self.status.remove(CpuFlags::NEGATIVE);
        }
    }

    fn add_to_register_a(&mut self, data: u8) {
        let carry_in: u16 = if self.status.contains(CpuFlags::CARRY) {
            1
        } else {
            0
        };

        let sum = self.register_a as u16 + data as u16 + carry_in;

        let carry_out = sum > 0xFF;
        if carry_out {
            self.status.insert(CpuFlags::CARRY);
        } else {
            self.status.remove(CpuFlags::CARRY);
        }

        let result = sum as u8;

        let is_sign_of_inputs_different_from_sign_of_result =
            (data ^ result) & (result ^ self.register_a) & 0x80 != 0;

        if is_sign_of_inputs_different_from_sign_of_result {
            self.set_overflow_flag();
        } else {
            self.clear_overflow_flag();
        }

        self.set_register_a(result);
    }

    fn sub_to_register_a(&mut self, data: u8) {
        self.add_to_register_a((data as i8).wrapping_neg().wrapping_sub(1) as u8);
    }

    fn and_with_register_a(&mut self, data: u8) {
        self.set_register_a(data & self.register_a);
    }

    fn or_with_register_a(&mut self, data: u8) {
        self.set_register_a(data | self.register_a);
    }

    fn xor_with_register_a(&mut self, data: u8) {
        self.set_register_a(data ^ self.register_a);
    }

    fn set_register_a(&mut self, value: u8) {
        self.register_a = value;
        self.update_zero_flag(self.register_a);
        self.update_negative_flag(self.register_a);
    }

    fn set_register_x(&mut self, value: u8) {
        self.register_x = value;
        self.update_zero_flag(self.register_x);
        self.update_negative_flag(self.register_x);
    }

    fn set_register_y(&mut self, value: u8) {
        self.register_y = value;
        self.update_zero_flag(self.register_y);
        self.update_negative_flag(self.register_y);
    }

    fn set_carry_flag(&mut self) {
        self.status.insert(CpuFlags::CARRY);
    }

    fn clear_carry_flag(&mut self) {
        self.status.remove(CpuFlags::CARRY);
    }

    fn set_decimal_mode_flag(&mut self) {
        self.status.insert(CpuFlags::DECIMAL_MODE);
    }

    fn clear_decimal_mode_flag(&mut self) {
        self.status.remove(CpuFlags::DECIMAL_MODE);
    }

    fn set_interrupt_disable_flag(&mut self) {
        self.status.insert(CpuFlags::INTERRUPT_DISABLE);
    }

    fn clear_interrupt_disable_flag(&mut self) {
        self.status.remove(CpuFlags::INTERRUPT_DISABLE);
    }

    fn set_overflow_flag(&mut self) {
        self.status.insert(CpuFlags::OVERFLOW);
    }

    fn clear_overflow_flag(&mut self) {
        self.status.remove(CpuFlags::OVERFLOW);
    }

    fn compute_operand_address(&mut self, mode: &AddressingMode) -> (u16, bool) {
        match mode {
            AddressingMode::Immediate => (self.program_counter, false),
            _ => self.compute_real_address(mode, self.program_counter),
        }
    }

    pub fn compute_real_address(&mut self, mode: &AddressingMode, addr: u16) -> (u16, bool) {
        match mode {
            AddressingMode::ZeroPage => (self.mem_read(addr) as u16, false),
            AddressingMode::ZeroPageX => {
                let pos = self.mem_read(addr);
                let addr = pos.wrapping_add(self.register_x) as u16;
                (addr, false)
            }
            AddressingMode::ZeroPageY => {
                let pos = self.mem_read(addr);
                let addr = pos.wrapping_add(self.register_y) as u16;
                (addr, false)
            }
            AddressingMode::Absolute => (self.mem_read_u16(addr), false),
            AddressingMode::AbsoluteX => {
                let base = self.mem_read_u16(addr);
                let addr = base.wrapping_add(self.register_x as u16);
                (addr, self.page_cross(base, addr))
            }
            AddressingMode::AbsoluteY => {
                let base = self.mem_read_u16(addr);
                let addr = base.wrapping_add(self.register_y as u16);
                (addr, self.page_cross(base, addr))
            }
            AddressingMode::IndirectX => {
                let base = self.mem_read(addr);

                let ptr = base.wrapping_add(self.register_x);
                let lo = self.mem_read(ptr as u16);
                let hi = self.mem_read(ptr.wrapping_add(1) as u16);
                ((hi as u16) << 8 | (lo as u16), false)
            }
            AddressingMode::IndirectY => {
                let base = self.mem_read(addr);

                let lo = self.mem_read(base as u16);
                let hi = self.mem_read((base as u8).wrapping_add(1) as u16);
                let deref_base = (hi as u16) << 8 | (lo as u16);
                let deref = deref_base.wrapping_add(self.register_y as u16);
                (deref, self.page_cross(deref, deref_base))
            }
            _ => {
                panic!("Memory addressing mode {:?} is not supported", mode);
            }
        }
    }

    fn page_cross(&self, addr1: u16, addr2: u16) -> bool {
        addr1 & 0xFF00 != addr2 & 0xFF00
    }

    fn manage_interrupt(&mut self, interrupt: interrupt::Interrupt) {
        self.stack_push_u16(self.program_counter);
        let mut status_flags = self.status.clone();
        status_flags.set(CpuFlags::BREAK, interrupt.b_flag_mask & FOURTH_BIT == 1);
        status_flags.set(CpuFlags::BREAK2, interrupt.b_flag_mask & FIFTH_BIT == 1);

        self.stack_push(status_flags.bits());
        self.status.insert(CpuFlags::INTERRUPT_DISABLE);

        self.bus.tick(interrupt.cpu_cycles);
        self.program_counter = self.mem_read_u16(interrupt.vec_addr);
    }

    pub fn program_counter(&self) -> u16 {
        self.program_counter
    }

    pub fn register_a(&self) -> u8 {
        self.register_a
    }

    pub fn register_x(&self) -> u8 {
        self.register_x
    }

    pub fn register_y(&self) -> u8 {
        self.register_y
    }

    pub fn status(&self) -> CpuFlags {
        self.status
    }

    pub fn stack_pointer(&self) -> u8 {
        self.stack_pointer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nes::cartridge::tests;
    use crate::nes::ppu::Ppu;

    #[test]
    fn test_0xa9_lda_immediate_load_data() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0x05, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_a, 0x05);
        assert_eq!(cpu.status.bits() & 0b0000_0010, 0b00);
        assert_eq!(cpu.status.bits() & 0b1000_0000, 0);
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0x00, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.status.bits() & 0b0000_0010, 0b10);
    }

    #[test]
    fn test_0xa9_lda_negative_flag() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0xFF, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.status.bits() & 0b1000_0000, 0b1000_0000);
    }

    #[test]
    fn test_lda_zero_page() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA5, 0x10, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.mem_write(0x10, 0x55);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_a, 0x55);
    }

    #[test]
    fn test_lda_zero_page_x() {
        let rom =
            tests::create_simple_test_rom_with_data(vec![0xA9, 0x0F, 0xAA, 0xB5, 0x80, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.mem_write(0x8F, 0x55);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_a, 0x55);
    }

    #[test]
    fn test_lda_absolute() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xAD, 0x8F, 0x00, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.mem_write(0x008F, 0x55);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_a, 0x55);
    }

    #[test]
    fn test_lda_absolute_x() {
        let rom = tests::create_simple_test_rom_with_data(
            vec![0xA9, 0x0F, 0xAA, 0xBD, 0x80, 0x00, 0x00],
            None,
        );
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.mem_write(0x008F, 0x55);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_a, 0x55);
    }

    #[test]
    fn test_lda_indirect_x() {
        let rom = tests::create_simple_test_rom_with_data(
            vec![0xA9, 0x0F, 0xAA, 0xA1, 0x80, 0x00, 0x00],
            None,
        );
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.mem_write(0x008F, 0x55);
        cpu.mem_write(0x0055, 0x0A);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_a, 0x0A);
    }

    #[test]
    fn test_0x69_adc_add_with_carry() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0x01, 0x69, 0x01, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_a, 2);
    }

    #[test]
    fn test_0x69_adc_add_with_carry_overflow() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0x7F, 0x69, 0x7F, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_a, 0xFE);
        assert!(cpu.status.contains(CpuFlags::OVERFLOW));
    }

    #[test]
    fn test_0x29_and_logical_and() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0x99, 0x29, 0x91, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_a, 0x91);
    }

    #[test]
    fn test_0x06_asl_arithmetic_shift_left() {
        let rom = tests::create_simple_test_rom_with_data(vec![0x06, 0x10, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.mem_write(0x10, 0x02);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.mem_read(0x10), 0x04);
    }

    #[test]
    fn test_0x0a_asl_arithmetic_shift_left_accumulator() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0x01, 0x0A, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_a, 0x02);
    }

    #[test]
    fn test_0x24_bit_bit_test() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0x01, 0x24, 0x10, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.mem_write(0x10, 0x01);
        cpu.reset();
        cpu.run();
        assert!(!cpu.status.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0xc9_cmp_compare() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0x01, 0xC9, 0x01, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert!(cpu.status.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0xc6_dec_decrement_memory() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0x01, 0xC6, 0x10, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.mem_write(0x10, 0x01);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.mem_read(0x10), 0x00);
    }

    #[test]
    fn test_0x49_eor_exclusive_or() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0x01, 0x49, 0x10, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_a, 0x11);
    }

    #[test]
    fn test_0x46_lsr_logical_shift_left() {
        let rom = tests::create_simple_test_rom_with_data(vec![0x46, 0x10, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.mem_write(0x10, 0x10);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.mem_read(0x10), 0b00001000);
    }

    #[test]
    fn test_0x4a_lsr_logical_shift_left_accumulator() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0x10, 0x4A, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_a, 0b00001000);
    }

    #[test]
    fn test_0x09_ora_logical_inclusive_or() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0x10, 0x09, 0x0F, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_a, 0x1F);
    }

    #[test]
    fn test_0x26_rol_rotate_left() {
        let rom = tests::create_simple_test_rom_with_data(vec![0x26, 0x10, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.mem_write(0x10, 0x80);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.mem_read(0x10), 0x00);
        assert!(cpu.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x2a_rol_rotate_left_accumulator() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0x80, 0x2A, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_a, 0x00);
        assert!(cpu.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x66_ror_rotate_right() {
        let rom = tests::create_simple_test_rom_with_data(vec![0x66, 0x10, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.mem_write(0x10, 0x01);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.mem_read(0x10), 0x00);
        assert!(cpu.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x6a_ror_rotate_right_accumulator() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0x80, 0x6A, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_a, 0x40);
    }

    #[test]
    fn test_0xe9_sbc_subtract_with_carry() {
        // carry is set before the operation
        let rom =
            tests::create_simple_test_rom_with_data(vec![0xA9, 0x01, 0x38, 0xE9, 0x02, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_a as i8, -1);
    }

    #[test]
    fn test_branching() {
        let rom = tests::create_simple_test_rom_with_data(
            vec![0xA9, 0x01, 0x10, 0x02, 0xA9, 0xFF, 0xA9, 0x00, 0x00],
            None,
        );
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_a, 0x00);
    }

    #[test]
    fn test_0xca_dex_decrement_x() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0x01, 0xAA, 0xCA, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_x, 0x00);
        assert_eq!(cpu.register_a, 0x01);
    }

    #[test]
    fn test_0x88_dey_decrement_y() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0x02, 0xA8, 0x88, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_y, 0x01);
        assert_eq!(cpu.register_a, 0x02);
    }

    #[test]
    fn test_0xe6_inc_increment_memory() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xE6, 0x10, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.mem_write(0x10, 0x01);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.mem_read(0x10), 0x02);
    }

    #[test]
    fn test_0xe8_inx_increment_x() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0x0A, 0xAA, 0xE8, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_x, 11);
    }

    #[test]
    fn test_0xe8_inx_increment_x_overflow() {
        let rom =
            tests::create_simple_test_rom_with_data(vec![0xA9, 0xFF, 0xAA, 0xE8, 0xE8, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_x, 1);
    }

    #[test]
    fn test_0xe8_inx_increment_x_zero_flag() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0xFF, 0xAA, 0xE8, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.status.bits() & 0b0000_0010, 0b10);
    }

    #[test]
    fn test_0xe8_inx_increment_x_negative_flag() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0xFE, 0xAA, 0xE8, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.status.bits() & 0b1000_0000, 0b1000_0000);
    }

    #[test]
    fn test_0xc8_iny_increment_y() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0x0A, 0xA8, 0xC8, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_y, 11);
    }

    #[test]
    fn test_0xa2_ldx_load_register_x() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA2, 0x0A, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_x, 10);
    }

    #[test]
    fn test_0xa0_ldy_load_register_y() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA0, 0x0A, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_y, 10);
    }

    #[test]
    fn test_0x85_sta_store_register_a() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0x0A, 0x85, 0x10, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.mem_read(0x10), 0x0A);
    }

    #[test]
    fn test_0x86_stx_store_register_x() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA2, 0x0A, 0x86, 0x10, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.mem_read(0x10), 0x0A);
    }

    #[test]
    fn test_0x84_sty_store_register_y() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA0, 0x0A, 0x84, 0x10, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.mem_read(0x10), 0x0A);
    }

    #[test]
    fn test_0xaa_tax_move_a_to_x() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0x0A, 0xAA, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_x, 10);
    }

    #[test]
    fn test_0xaa_tax_move_a_to_x_zero_flag() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0x00, 0xAA, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.status.bits() & 0b0000_0010, 0b10);
    }

    #[test]
    fn test_0xaa_tax_move_a_to_x_negative_flag() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0xFF, 0xAA, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.status.bits() & 0b1000_0000, 0b1000_0000);
    }

    #[test]
    fn test_0xa8_tay_move_a_to_y() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0x0A, 0xA8, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_y, 10);
    }

    #[test]
    fn test_0x8a_txa_move_x_to_a() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA2, 0x0A, 0x8A, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_a, 10);
    }

    #[test]
    fn test_0x98_tya_move_y_to_a() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA0, 0x0A, 0x98, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_a, 10);
    }

    #[test]
    fn test_0xc7_dcp_unofficial() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xC7, 0x10, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.mem_write(0x10, 0x01);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.mem_read(0x10), 0x00);
    }

    #[test]
    fn test_0x27_rla_unofficial() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0xFF, 0x27, 0x10, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.mem_write(0x10, 0x01);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_a, 0x02);
    }

    #[test]
    fn test_0x07_slo_unofficial() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0x00, 0x07, 0x10, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.mem_write(0x10, 0x01);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_a, 0x02);
    }

    #[test]
    fn test_0x47_sre_unofficial() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0xFF, 0x47, 0x10, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.mem_write(0x10, 0x02);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_a, 0xFE);
    }

    #[test]
    fn test_0xcb_axs_unofficial() {
        let rom = tests::create_simple_test_rom_with_data(
            vec![0xA9, 0xFF, 0xA2, 0x0F, 0xCB, 0x02, 0x00],
            None,
        );
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_x, 0x0D);
    }

    #[test]
    fn test_0x6b_arr_unofficial() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0xFE, 0x6B, 0x0F, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_a, 0x07);
        assert!(!cpu.status.contains(CpuFlags::CARRY));
        assert!(!cpu.status.contains(CpuFlags::OVERFLOW));
    }

    #[test]
    fn test_0xeb_sbc_unofficial() {
        let rom =
            tests::create_simple_test_rom_with_data(vec![0xA9, 0x02, 0x38, 0xEB, 0x01, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_a, 0x01);
    }

    #[test]
    fn test_0x0b_anc_unofficial() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0xF2, 0x0B, 0xF1, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_a, 0xF0);
        assert!(cpu.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x4b_alr_unofficial() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0xF2, 0x4B, 0xF1, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_a, 0x78);
    }

    #[test]
    fn test_0x67_rra_unofficial() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA9, 0x01, 0x67, 0x10, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.mem_write(0x10, 0x10);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.mem_read(0x10), 0x08);
        assert_eq!(cpu.register_a, 0x09);
    }

    #[test]
    fn test_0xe7_isb_unofficial() {
        let rom =
            tests::create_simple_test_rom_with_data(vec![0xA9, 0x02, 0x38, 0xE7, 0x10, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.mem_write(0x10, 0x01);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.mem_read(0x10), 0x02);
        assert_eq!(cpu.register_a, 0x00);
    }

    #[test]
    fn test_0xa7_lax_unofficial() {
        let rom = tests::create_simple_test_rom_with_data(vec![0xA7, 0x10, 0x00], None);
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.mem_write(0x10, 0x01);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.register_a, 0x01);
        assert_eq!(cpu.register_x, 0x01);
    }

    #[test]
    fn test_0x87_sax_unofficial() {
        let rom = tests::create_simple_test_rom_with_data(
            vec![0xA9, 0xFF, 0xA2, 0xFE, 0x87, 0x10, 0x00],
            None,
        );
        let bus = Bus::new(rom, |ppu: &Ppu| {});
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.mem_read(0x10), 0xFE);
    }
}
