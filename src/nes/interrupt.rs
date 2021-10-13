/// NES' interrupts

#[derive(Eq, PartialEq)]
pub enum InterruptType {
    Nmi,
}

#[derive(Eq, PartialEq)]
pub struct Interrupt {
    pub itype: InterruptType,
    pub vec_addr: u16,
    pub b_flag_mask: u8,
    pub cpu_cycles: u8,
}

pub const NMI: Interrupt = Interrupt {
    itype: InterruptType::Nmi,
    vec_addr: 0xFFFA,
    b_flag_mask: 0b00100000,
    cpu_cycles: 2,
};