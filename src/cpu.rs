use crate::nes::Nes;
use crate::Emu;

mod addressing_mode;
mod instruction;

#[derive(Debug, Default)]
pub struct Cpu {
    a: u8,
    x: u8,
    y: u8,
    s: u8,
    p: Status,
    pc: u16,
}

bitflags! {
    #[derive(Default)]
    struct Status: u8 {
        // Carry
        const C = 1;
        // Zero
        const Z = 1 << 1;
        // Interrupt Disable
        const I = 1 << 2;
        // Decimal
        const D = 1 << 3;
        // Overflow
        const V = 1 << 6;
        // Negative
        const N = 1 << 7;
        // B flags
        const INTERRUPT_B = 0b00100000;
        const INSTRUCTION_B = 0b00110000;
    }
}

impl Status {
    fn set_zn(&mut self, v: u8) {
        self.set(Self::Z, v == 0);
        self.set(Self::N, v & 0x80 == 0x80);
    }
}

pub(crate) trait CpuBus {
    fn read(nes: &mut Nes, addr: u16) -> u8;
    fn write(nes: &mut Nes, addr: u16, value: u8);
}

pub(crate) trait CpuTick {
    fn tick(nes: &mut Nes);
    fn tick_n(nes: &mut Nes, n: u128);
}

struct CpuBusInternal<B: CpuBus, T: CpuTick> {
    _bus: std::marker::PhantomData<B>,
    _tick: std::marker::PhantomData<T>,
}

impl<B: CpuBus, T: CpuTick> CpuBus for CpuBusInternal<B, T> {
    fn read(nes: &mut Nes, addr: u16) -> u8 {
        let v = B::read(nes, addr);
        T::tick(nes);
        v
    }

    fn write(nes: &mut Nes, addr: u16, value: u8) {
        //TODO OAMDMA
        B::write(nes, addr, value);
        T::tick(nes);
    }
}

fn read<B: CpuBus, T: CpuTick>(nes: &mut Nes, addr: u16) -> u8 {
    CpuBusInternal::<B, T>::read(nes, addr)
}

fn write<B: CpuBus, T: CpuTick>(nes: &mut Nes, addr: u16, value: u8) {
    CpuBusInternal::<B, T>::write(nes, addr, value)
}

fn read_word<B: CpuBus, T: CpuTick>(nes: &mut Nes, addr: u16) -> u16 {
    CpuBusInternal::<B, T>::read(nes, addr) as u16
        | (CpuBusInternal::<B, T>::read(nes, addr + 1) as u16) << 8
}

fn read_on_indirect<B: CpuBus, T: CpuTick>(nes: &mut Nes, addr: u16) -> u16 {
    let low = CpuBusInternal::<B, T>::read(nes, addr) as u16;
    // Reproduce 6502 bug - http://nesdev.com/6502bugs.txt
    let high = CpuBusInternal::<B, T>::read(nes, (addr & 0xFF00) | ((addr + 1) & 0x00FF)) as u16;
    low | (high << 8)
}

fn push_stack<B: CpuBus, T: CpuTick>(nes: &mut Nes, v: u8) {
    write::<B, T>(nes, nes.cpu.s as u16, v);
    nes.cpu.s = nes.cpu.s.wrapping_sub(1);
}

fn pull_stack<B: CpuBus, T: CpuTick>(nes: &mut Nes) -> u8 {
    nes.cpu.s = nes.cpu.s.wrapping_add(1);
    read::<B, T>(nes, nes.cpu.s as u16)
}

fn push_stack_word<B: CpuBus, T: CpuTick>(nes: &mut Nes, v: u16) {
    push_stack::<B, T>(nes, (v >> 8) as u8);
    push_stack::<B, T>(nes, (v & 0xFF) as u8);
}

fn pull_stack_word<B: CpuBus, T: CpuTick>(nes: &mut Nes) -> u16 {
    let low = pull_stack::<B, T>(nes) as u16;
    let high = pull_stack::<B, T>(nes) as u16;
    low | (high << 8)
}

impl Emu {
    #[allow(dead_code)]
    fn cpu_step<B: CpuBus, T: CpuTick>(nes: &mut Nes) {
        let opcode = read::<B, T>(nes, nes.cpu.pc);
        nes.cpu.pc = nes.cpu.pc.wrapping_add(1);

        let inst = decode(opcode);

        let (_, addressing_mode) = &inst;
        let operand = addressing_mode::get_operand::<B, T>(nes, *addressing_mode);

        instruction::execute::<B, T>(nes, inst, operand);
    }
}

fn page_crossed(a: u16, b: u16) -> bool {
    a.wrapping_add(b) & 0xFF00 != (b & 0xFF00)
}

type Instruction = (Mnemonic, AddressingMode);

#[derive(Debug, Clone, Copy)]
#[rustfmt::skip]
enum AddressingMode {
    Implicit,
    Accumulator,
    Immediate,
    ZeroPage, ZeroPageX, ZeroPageY,
    Absolute,
    AbsoluteX { oops: bool },
    AbsoluteY { oops: bool },
    Relative,
    Indirect, IndexedIndirect, IndirectIndexed
}

#[derive(Debug)]
#[rustfmt::skip]
#[allow(dead_code, clippy::upper_case_acronyms)]
enum Mnemonic {
    // Load/Store Operations
    LDA, LDX, LDY, STA, STX, STY,
    // Register Operations
    TAX, TSX, TAY, TXA, TXS, TYA,
    // Stack instructions
    PHA, PHP, PLA, PLP,
    // Logical instructions
    AND, EOR, ORA, BIT,
    // Arithmetic instructions
    ADC, SBC, CMP, CPX, CPY,
    // Increment/Decrement instructions
    INC, INX, INY, DEC, DEX, DEY,
    // Shift instructions
    ASL, LSR, ROL, ROR,
    // Jump instructions
    JMP, JSR, RTS, RTI,
    // Branch instructions
    BCC, BCS, BEQ, BMI, BNE, BPL, BVC, BVS,
    // Flag control instructions
    CLC, CLD, CLI, CLV, SEC, SED, SEI,
    // Misc
    BRK, NOP,
    // Unofficial
    LAX, SAX, DCP, ISB, SLO, RLA, SRE, RRA,
}

fn decode(opcode: u8) -> Instruction {
    match opcode {
        0x69 => (Mnemonic::ADC, AddressingMode::Immediate),
        0x65 => (Mnemonic::ADC, AddressingMode::ZeroPage),
        0x75 => (Mnemonic::ADC, AddressingMode::ZeroPageX),
        0x6D => (Mnemonic::ADC, AddressingMode::Absolute),
        0x7D => (Mnemonic::ADC, AddressingMode::AbsoluteX { oops: true }),
        0x79 => (Mnemonic::ADC, AddressingMode::AbsoluteY { oops: true }),
        0x61 => (Mnemonic::ADC, AddressingMode::IndexedIndirect),
        0x71 => (Mnemonic::ADC, AddressingMode::IndirectIndexed),

        0x29 => (Mnemonic::AND, AddressingMode::Immediate),
        0x25 => (Mnemonic::AND, AddressingMode::ZeroPage),
        0x35 => (Mnemonic::AND, AddressingMode::ZeroPageX),
        0x2D => (Mnemonic::AND, AddressingMode::Absolute),
        0x3D => (Mnemonic::AND, AddressingMode::AbsoluteX { oops: true }),
        0x39 => (Mnemonic::AND, AddressingMode::AbsoluteY { oops: true }),
        0x21 => (Mnemonic::AND, AddressingMode::IndexedIndirect),
        0x31 => (Mnemonic::AND, AddressingMode::IndirectIndexed),

        0x0A => (Mnemonic::ASL, AddressingMode::Accumulator),
        0x06 => (Mnemonic::ASL, AddressingMode::ZeroPage),
        0x16 => (Mnemonic::ASL, AddressingMode::ZeroPageX),
        0x0E => (Mnemonic::ASL, AddressingMode::Absolute),
        0x1E => (Mnemonic::ASL, AddressingMode::AbsoluteX { oops: false }),

        0x90 => (Mnemonic::BCC, AddressingMode::Relative),
        0xB0 => (Mnemonic::BCS, AddressingMode::Relative),
        0xF0 => (Mnemonic::BEQ, AddressingMode::Relative),

        0x24 => (Mnemonic::BIT, AddressingMode::ZeroPage),
        0x2C => (Mnemonic::BIT, AddressingMode::Absolute),

        0x30 => (Mnemonic::BMI, AddressingMode::Relative),
        0xD0 => (Mnemonic::BNE, AddressingMode::Relative),
        0x10 => (Mnemonic::BPL, AddressingMode::Relative),

        0x00 => (Mnemonic::BRK, AddressingMode::Implicit),

        0x50 => (Mnemonic::BVC, AddressingMode::Relative),
        0x70 => (Mnemonic::BVS, AddressingMode::Relative),

        0x18 => (Mnemonic::CLC, AddressingMode::Implicit),
        0xD8 => (Mnemonic::CLD, AddressingMode::Implicit),
        0x58 => (Mnemonic::CLI, AddressingMode::Implicit),
        0xB8 => (Mnemonic::CLV, AddressingMode::Implicit),

        0xC9 => (Mnemonic::CMP, AddressingMode::Immediate),
        0xC5 => (Mnemonic::CMP, AddressingMode::ZeroPage),
        0xD5 => (Mnemonic::CMP, AddressingMode::ZeroPageX),
        0xCD => (Mnemonic::CMP, AddressingMode::Absolute),
        0xDD => (Mnemonic::CMP, AddressingMode::AbsoluteX { oops: true }),
        0xD9 => (Mnemonic::CMP, AddressingMode::AbsoluteY { oops: false }),
        0xC1 => (Mnemonic::CMP, AddressingMode::IndexedIndirect),
        0xD1 => (Mnemonic::CMP, AddressingMode::IndirectIndexed),

        0xE0 => (Mnemonic::CPX, AddressingMode::Immediate),
        0xE4 => (Mnemonic::CPX, AddressingMode::ZeroPage),
        0xEC => (Mnemonic::CPX, AddressingMode::Absolute),
        0xC0 => (Mnemonic::CPY, AddressingMode::Immediate),
        0xC4 => (Mnemonic::CPY, AddressingMode::ZeroPage),
        0xCC => (Mnemonic::CPY, AddressingMode::Absolute),

        0xC6 => (Mnemonic::DEC, AddressingMode::ZeroPage),
        0xD6 => (Mnemonic::DEC, AddressingMode::ZeroPageX),
        0xCE => (Mnemonic::DEC, AddressingMode::Absolute),
        0xDE => (Mnemonic::DEC, AddressingMode::AbsoluteX { oops: false }),

        0xCA => (Mnemonic::DEX, AddressingMode::Implicit),
        0x88 => (Mnemonic::DEY, AddressingMode::Implicit),

        0x49 => (Mnemonic::EOR, AddressingMode::Immediate),
        0x45 => (Mnemonic::EOR, AddressingMode::ZeroPage),
        0x55 => (Mnemonic::EOR, AddressingMode::ZeroPageX),
        0x4D => (Mnemonic::EOR, AddressingMode::Absolute),
        0x5D => (Mnemonic::EOR, AddressingMode::AbsoluteX { oops: true }),
        0x59 => (Mnemonic::EOR, AddressingMode::AbsoluteY { oops: true }),
        0x41 => (Mnemonic::EOR, AddressingMode::IndexedIndirect),
        0x51 => (Mnemonic::EOR, AddressingMode::IndirectIndexed),

        0xE6 => (Mnemonic::INC, AddressingMode::ZeroPage),
        0xF6 => (Mnemonic::INC, AddressingMode::ZeroPageX),
        0xEE => (Mnemonic::INC, AddressingMode::Absolute),
        0xFE => (Mnemonic::INC, AddressingMode::AbsoluteX { oops: false }),

        0xE8 => (Mnemonic::INX, AddressingMode::Implicit),
        0xC8 => (Mnemonic::INY, AddressingMode::Implicit),

        0x4C => (Mnemonic::JMP, AddressingMode::Absolute),
        0x6C => (Mnemonic::JMP, AddressingMode::Indirect),

        0x20 => (Mnemonic::JSR, AddressingMode::Absolute),

        0xA9 => (Mnemonic::LDA, AddressingMode::Immediate),
        0xA5 => (Mnemonic::LDA, AddressingMode::ZeroPage),
        0xB5 => (Mnemonic::LDA, AddressingMode::ZeroPageX),
        0xAD => (Mnemonic::LDA, AddressingMode::Absolute),
        0xBD => (Mnemonic::LDA, AddressingMode::AbsoluteX { oops: true }),
        0xB9 => (Mnemonic::LDA, AddressingMode::AbsoluteY { oops: true }),
        0xA1 => (Mnemonic::LDA, AddressingMode::IndexedIndirect),
        0xB1 => (Mnemonic::LDA, AddressingMode::IndirectIndexed),

        0xA2 => (Mnemonic::LDX, AddressingMode::Immediate),
        0xA6 => (Mnemonic::LDX, AddressingMode::ZeroPage),
        0xB6 => (Mnemonic::LDX, AddressingMode::ZeroPageY),
        0xAE => (Mnemonic::LDX, AddressingMode::Absolute),
        0xBE => (Mnemonic::LDX, AddressingMode::AbsoluteY { oops: true }),

        0xA0 => (Mnemonic::LDY, AddressingMode::Immediate),
        0xA4 => (Mnemonic::LDY, AddressingMode::ZeroPage),
        0xB4 => (Mnemonic::LDY, AddressingMode::ZeroPageX),
        0xAC => (Mnemonic::LDY, AddressingMode::Absolute),
        0xBC => (Mnemonic::LDY, AddressingMode::AbsoluteY { oops: true }),

        0x4A => (Mnemonic::LSR, AddressingMode::Accumulator),
        0x46 => (Mnemonic::LSR, AddressingMode::ZeroPage),
        0x56 => (Mnemonic::LSR, AddressingMode::ZeroPageX),
        0x4E => (Mnemonic::LSR, AddressingMode::Absolute),
        0x5E => (Mnemonic::LSR, AddressingMode::AbsoluteX { oops: false }),

        0xEA => (Mnemonic::NOP, AddressingMode::Implicit),

        0x09 => (Mnemonic::ORA, AddressingMode::Immediate),
        0x05 => (Mnemonic::ORA, AddressingMode::ZeroPage),
        0x15 => (Mnemonic::ORA, AddressingMode::ZeroPageX),
        0x0D => (Mnemonic::ORA, AddressingMode::Absolute),
        0x1D => (Mnemonic::ORA, AddressingMode::AbsoluteX { oops: true }),
        0x19 => (Mnemonic::ORA, AddressingMode::AbsoluteY { oops: true }),
        0x01 => (Mnemonic::ORA, AddressingMode::IndexedIndirect),
        0x11 => (Mnemonic::ORA, AddressingMode::IndirectIndexed),

        0x48 => (Mnemonic::PHA, AddressingMode::Implicit),
        0x08 => (Mnemonic::PHP, AddressingMode::Implicit),
        0x68 => (Mnemonic::PLA, AddressingMode::Implicit),
        0x28 => (Mnemonic::PLP, AddressingMode::Implicit),

        0x2A => (Mnemonic::ROL, AddressingMode::Accumulator),
        0x26 => (Mnemonic::ROL, AddressingMode::ZeroPage),
        0x36 => (Mnemonic::ROL, AddressingMode::ZeroPageX),
        0x2E => (Mnemonic::ROL, AddressingMode::Absolute),
        0x3E => (Mnemonic::ROL, AddressingMode::AbsoluteX { oops: false }),

        0x6A => (Mnemonic::ROR, AddressingMode::Accumulator),
        0x66 => (Mnemonic::ROR, AddressingMode::ZeroPage),
        0x76 => (Mnemonic::ROR, AddressingMode::ZeroPageX),
        0x6E => (Mnemonic::ROR, AddressingMode::Absolute),
        0x7E => (Mnemonic::ROR, AddressingMode::AbsoluteX { oops: false }),

        0x40 => (Mnemonic::RTI, AddressingMode::Implicit),
        0x60 => (Mnemonic::RTS, AddressingMode::Implicit),

        0xE9 => (Mnemonic::SBC, AddressingMode::Immediate),
        0xE5 => (Mnemonic::SBC, AddressingMode::ZeroPage),
        0xF5 => (Mnemonic::SBC, AddressingMode::ZeroPageX),
        0xED => (Mnemonic::SBC, AddressingMode::Absolute),
        0xFD => (Mnemonic::SBC, AddressingMode::AbsoluteX { oops: true }),
        0xF9 => (Mnemonic::SBC, AddressingMode::AbsoluteY { oops: true }),
        0xE1 => (Mnemonic::SBC, AddressingMode::IndexedIndirect),
        0xF1 => (Mnemonic::SBC, AddressingMode::IndirectIndexed),

        0x38 => (Mnemonic::SEC, AddressingMode::Implicit),
        0xF8 => (Mnemonic::SED, AddressingMode::Implicit),
        0x78 => (Mnemonic::SEI, AddressingMode::Implicit),

        0x85 => (Mnemonic::STA, AddressingMode::ZeroPage),
        0x95 => (Mnemonic::STA, AddressingMode::ZeroPageX),
        0x8D => (Mnemonic::STA, AddressingMode::Absolute),
        0x9D => (Mnemonic::STA, AddressingMode::AbsoluteX { oops: false }),
        0x99 => (Mnemonic::STA, AddressingMode::AbsoluteY { oops: false }),
        0x81 => (Mnemonic::STA, AddressingMode::ZeroPage),
        0x91 => (Mnemonic::STA, AddressingMode::ZeroPage),

        0x86 => (Mnemonic::STX, AddressingMode::ZeroPage),
        0x96 => (Mnemonic::STX, AddressingMode::ZeroPageY),
        0x8E => (Mnemonic::STX, AddressingMode::Absolute),
        0x84 => (Mnemonic::STY, AddressingMode::ZeroPage),
        0x94 => (Mnemonic::STY, AddressingMode::ZeroPageX),
        0x8C => (Mnemonic::STY, AddressingMode::Absolute),

        0xAA => (Mnemonic::TAX, AddressingMode::Implicit),
        0xA8 => (Mnemonic::TAY, AddressingMode::Implicit),
        0xBA => (Mnemonic::TSX, AddressingMode::Implicit),
        0x8A => (Mnemonic::TXA, AddressingMode::Implicit),
        0x9A => (Mnemonic::TXS, AddressingMode::Implicit),
        0x98 => (Mnemonic::TYA, AddressingMode::Implicit),

        _ => (Mnemonic::NOP, AddressingMode::Implicit),
    }
}

#[cfg(test)]
mod test_mock {
    use super::*;

    pub(super) struct CpuTickMock {}
    impl CpuTick for CpuTickMock {
        fn tick(nes: &mut Nes) {
            nes.cpu_cycles = nes.cpu_cycles.wrapping_add(1);
        }
        fn tick_n(nes: &mut Nes, n: u128) {
            nes.cpu_cycles = nes.cpu_cycles.wrapping_add(n);
        }
    }

    pub(super) struct CpuBusMock {}
    impl CpuBus for CpuBusMock {
        fn read(nes: &mut Nes, addr: u16) -> u8 {
            nes.wram[addr as usize]
        }
        fn write(nes: &mut Nes, addr: u16, value: u8) {
            nes.wram[addr as usize] = value
        }
    }
}

#[cfg(test)]
mod instruction_tests {
    use super::test_mock::*;
    use super::*;

    #[test]
    fn load_store_operations() {
        // LDA
        {
            let mut nes = Nes::new();
            nes.cpu.pc = 0x020F;
            nes.wram[0x020F] = 0xA9;
            nes.wram[0x0210] = 0x31;

            Emu::cpu_step::<CpuBusMock, CpuTickMock>(&mut nes);
            assert_eq!(nes.cpu.a, 0x31);
            assert_eq!(nes.cpu_cycles, 2);
            assert_eq!(nes.cpu.p, Status::empty());
        }
        // STA
        {
            let mut nes = Nes::new();
            nes.cpu.pc = 0x020F;
            nes.wram[0x020F] = 0x8D;
            nes.wram[0x0210] = 0x19;
            nes.wram[0x0211] = 0x04;
            nes.cpu.a = 0x91;

            Emu::cpu_step::<CpuBusMock, CpuTickMock>(&mut nes);
            assert_eq!(CpuBusMock::read(&mut nes, 0x0419), 0x91);
            assert_eq!(nes.cpu_cycles, 4);
        }
    }

    #[test]
    fn register_transfers() {
        // TAX
        {
            let mut nes = Nes::new();
            nes.cpu.pc = 0x020F;
            nes.wram[0x020F] = 0xAA;
            nes.cpu.a = 0x83;

            Emu::cpu_step::<CpuBusMock, CpuTickMock>(&mut nes);
            assert_eq!(nes.cpu.x, 0x83);
            assert_eq!(nes.cpu_cycles, 2);
            assert_eq!(nes.cpu.p, Status::N);
        }
        // TYA
        {
            let mut nes = Nes::new();
            nes.cpu.pc = 0x020F;
            nes.wram[0x020F] = 0x98;
            nes.cpu.y = 0xF0;

            Emu::cpu_step::<CpuBusMock, CpuTickMock>(&mut nes);
            assert_eq!(nes.cpu.a, 0xF0);
            assert_eq!(nes.cpu_cycles, 2);
            assert_eq!(nes.cpu.p, Status::N);
        }
    }

    #[test]
    fn stack_operations() {
        // TSX
        {
            let mut nes = Nes::new();
            nes.cpu.pc = 0x020F;
            nes.wram[0x020F] = 0xBA;
            nes.cpu.s = 0xF3;

            Emu::cpu_step::<CpuBusMock, CpuTickMock>(&mut nes);
            assert_eq!(nes.cpu.x, 0xF3);
            assert_eq!(nes.cpu_cycles, 2);
            assert_eq!(nes.cpu.p, Status::N);
        }
        // PHA
        {
            let mut nes = Nes::new();
            nes.cpu.pc = 0x020F;
            nes.wram[0x020F] = 0x48;
            nes.cpu.s = 0xFD;
            nes.cpu.a = 0x72;

            Emu::cpu_step::<CpuBusMock, CpuTickMock>(&mut nes);
            assert_eq!(nes.cpu.s, 0xFC);
            assert_eq!(CpuBusMock::read(&mut nes, 0x00FD), 0x72);
            assert_eq!(nes.cpu_cycles, 3);
        }
        // PHP
        {
            let mut nes = Nes::new();
            nes.cpu.pc = 0x020F;
            nes.wram[0x020F] = 0x08;
            nes.cpu.s = 0xFD;
            nes.cpu.p = Status::N | Status::D | Status::C;

            Emu::cpu_step::<CpuBusMock, CpuTickMock>(&mut nes);
            assert_eq!(nes.cpu.s, 0xFC);
            assert_eq!(
                CpuBusMock::read(&mut nes, 0x00FD),
                (nes.cpu.p | Status::INSTRUCTION_B).bits()
            );
            assert_eq!(nes.cpu_cycles, 3);
        }
        // PLP
        {
            let mut nes = Nes::new();
            nes.cpu.pc = 0x020F;
            nes.wram[0x020F] = 0x28;
            nes.cpu.s = 0xBF;
            nes.wram[0x00C0] = 0x7A;

            Emu::cpu_step::<CpuBusMock, CpuTickMock>(&mut nes);
            assert_eq!(nes.cpu.s, 0xC0);
            assert_eq!(nes.cpu.p.bits(), 0x4A);
            assert_eq!(nes.cpu_cycles, 4);
        }
    }

    #[test]
    fn logical() {
        // EOR
        {
            let mut nes = Nes::new();
            nes.cpu.pc = 0x020F;
            nes.wram[0x020F] = 0x49;
            nes.wram[0x0210] = 0x38;
            nes.cpu.a = 0x21;

            Emu::cpu_step::<CpuBusMock, CpuTickMock>(&mut nes);
            assert_eq!(nes.cpu.a, 0x19);
            assert_eq!(nes.cpu_cycles, 2);
            assert_eq!(nes.cpu.p, Status::empty());
        }
        // BIT
        {
            let mut nes = Nes::new();
            nes.cpu.pc = 0x020F;
            nes.wram[0x020F] = 0x2C;
            nes.wram[0x0210] = 0xB0;
            nes.wram[0x0211] = 0x03;
            nes.wram[0x03B0] = (Status::V | Status::N).bits();
            nes.cpu.a = 0x48;

            Emu::cpu_step::<CpuBusMock, CpuTickMock>(&mut nes);
            assert_eq!(nes.cpu_cycles, 4);
            assert_eq!(nes.cpu.p, Status::V);
        }
    }

    #[test]
    fn arithmetic() {
        // ADC
        {
            #[rustfmt::skip]
            let cases = [
                (0x50, 0x10, 0x60, Status::empty()),
                (0x50, 0x50, 0xA0, Status::N | Status::V),
                (0x50, 0x90, 0xE0, Status::N),
                (0x50, 0xD0, 0x20, Status::C),
                (0xD0, 0x10, 0xE0, Status::N),
                (0xD0, 0x50, 0x20, Status::C),
                (0xD0, 0x90, 0x60, Status::C | Status::V),
                (0xD0, 0xD0, 0xA0, Status::C | Status::N),
            ];

            for (i, (a, m, expected_a, expected_p)) in cases.iter().enumerate() {
                let mut nes = Nes::new();
                nes.cpu.pc = 0x020F;
                nes.wram[0x020F] = 0x6D;
                nes.wram[0x0210] = 0xD3;
                nes.wram[0x0211] = 0x04;
                nes.wram[0x04D3] = *m;
                nes.cpu.a = *a;

                Emu::cpu_step::<CpuBusMock, CpuTickMock>(&mut nes);
                assert_eq!(nes.cpu.a, *expected_a, "{}", i);
                assert_eq!(nes.cpu.p, *expected_p, "{}", i);
            }
        }
        // CPY
        {
            let mut nes = Nes::new();
            nes.cpu.pc = 0x020F;
            nes.wram[0x020F] = 0xCC;
            nes.wram[0x0210] = 0x36;
            nes.cpu.y = 0x37;

            Emu::cpu_step::<CpuBusMock, CpuTickMock>(&mut nes);
            assert_eq!(nes.cpu.p, Status::C)
        }
    }

    #[test]
    fn increments_and_decrements() {
        // INC
        {
            let mut nes = Nes::new();
            nes.cpu.pc = 0x020F;
            nes.wram[0x020F] = 0xEE;
            nes.wram[0x0210] = 0xD3;
            nes.wram[0x0211] = 0x04;
            nes.wram[0x04D3] = 0x7F;

            Emu::cpu_step::<CpuBusMock, CpuTickMock>(&mut nes);
            assert_eq!(CpuBusMock::read(&mut nes, 0x04D3), 0x80);
            assert_eq!(nes.cpu.p, Status::N);
        }
        // DEC
        {
            let mut nes = Nes::new();
            nes.cpu.pc = 0x020F;
            nes.wram[0x020F] = 0xCE;
            nes.wram[0x0210] = 0xD3;
            nes.wram[0x0211] = 0x04;
            nes.wram[0x04D3] = 0xC0;

            Emu::cpu_step::<CpuBusMock, CpuTickMock>(&mut nes);
            assert_eq!(CpuBusMock::read(&mut nes, 0x04D3), 0xBF);
            assert_eq!(nes.cpu.p, Status::N);
        }
    }

    #[test]
    fn shifts() {
        // ASL
        {
            let mut nes = Nes::new();
            nes.cpu.pc = 0x020F;
            nes.wram[0x020F] = 0x0A;
            nes.cpu.a = 0b10001010;

            Emu::cpu_step::<CpuBusMock, CpuTickMock>(&mut nes);
            assert_eq!(nes.cpu.a, 0b00010100);
            assert_eq!(nes.cpu.p, Status::C);
        }
        // ROL
        {
            let mut nes = Nes::new();
            nes.cpu.pc = 0x020F;
            nes.wram[0x020F] = 0x2A;
            nes.cpu.a = 0b10001010;
            nes.cpu.p = Status::C;

            Emu::cpu_step::<CpuBusMock, CpuTickMock>(&mut nes);
            assert_eq!(nes.cpu.a, 0b00010101);
            assert_eq!(nes.cpu.p, Status::C);
        }
        {
            let mut nes = Nes::new();
            nes.cpu.pc = 0x020F;
            nes.wram[0x020F] = 0x2A;
            nes.cpu.a = 0b10001010;
            nes.cpu.p = Status::N;

            Emu::cpu_step::<CpuBusMock, CpuTickMock>(&mut nes);
            assert_eq!(nes.cpu.a, 0b00010100);
            assert_eq!(nes.cpu.p, Status::C);
        }
    }

    #[test]
    fn calls() {
        // JSR
        {
            let mut nes = Nes::new();
            nes.cpu.pc = 0x020F;
            nes.wram[0x020F] = 0x20;
            nes.wram[0x0210] = 0x31;
            nes.wram[0x0211] = 0x40;
            nes.cpu.s = 0xBF;

            Emu::cpu_step::<CpuBusMock, CpuTickMock>(&mut nes);
            assert_eq!(nes.cpu.s, 0xBD);
            assert_eq!(nes.cpu.pc, 0x4031);
            assert_eq!(nes.cpu_cycles, 6);
            assert_eq!(CpuBusMock::read(&mut nes, 0xBE), 0x11);
            assert_eq!(CpuBusMock::read(&mut nes, 0xBF), 0x02);
        }
        // RTS
        {
            let mut nes = Nes::new();
            nes.cpu.pc = 0x0031;
            nes.wram[0x0031] = 0x60;

            nes.cpu.s = 0xBD;
            nes.wram[0x00BE] = 0x11;
            nes.wram[0x00BF] = 0x02;

            Emu::cpu_step::<CpuBusMock, CpuTickMock>(&mut nes);
            assert_eq!(nes.cpu.s, 0xBF);
            assert_eq!(nes.cpu.pc, 0x0211);
            assert_eq!(nes.cpu_cycles, 6);
        }
    }

    #[test]
    fn branches() {
        // BCC
        #[rustfmt::skip]
        let cases = [
            ("branch failed",               0x03, false, Status::N | Status::C, 2),
            ("branch succeed",              0x03, true, Status::N | Status::V, 3),
            ("branch succeed & new page",   0xD0, true, Status::N | Status::V, 4),
        ];
        for (name, operand, branch, p, expected_cycles) in cases {
            let mut nes = Nes::new();
            nes.cpu.pc = 0x0031;
            nes.wram[0x0031] = 0x90;
            nes.wram[0x0032] = operand;
            nes.cpu.p = p;

            Emu::cpu_step::<CpuBusMock, CpuTickMock>(&mut nes);
            if branch {
                assert_eq!(nes.cpu.pc, 0x33 + operand as u16, "{}", name);
            } else {
                assert_eq!(nes.cpu.pc, 0x33, "{}", name);
            }
            assert_eq!(nes.cpu_cycles, expected_cycles, "{}", name);
        }
    }

    #[test]
    fn status_flag_changes() {
        // CLD
        {
            let mut nes = Nes::new();
            nes.cpu.pc = 0x020F;
            nes.wram[0x020F] = 0xD8;
            nes.cpu.p = Status::V | Status::D | Status::C;

            Emu::cpu_step::<CpuBusMock, CpuTickMock>(&mut nes);
            assert_eq!(nes.cpu.pc, 0x0210);
            assert_eq!(nes.cpu_cycles, 2);
            assert_eq!(nes.cpu.p, Status::V | Status::C);
        }
        // SEI
        {
            let mut nes = Nes::new();
            nes.cpu.pc = 0x020F;
            nes.wram[0x020F] = 0x78;
            nes.cpu.p = Status::V | Status::D | Status::C;

            Emu::cpu_step::<CpuBusMock, CpuTickMock>(&mut nes);
            assert_eq!(nes.cpu.pc, 0x0210);
            assert_eq!(nes.cpu_cycles, 2);
            assert_eq!(nes.cpu.p, Status::V | Status::D | Status::C | Status::I);
        }
    }

    struct CpuBusMockForBRK {}
    impl CpuBus for CpuBusMockForBRK {
        fn read(nes: &mut Nes, addr: u16) -> u8 {
            if addr == 0xFFFE {
                return 0x23;
            }
            if addr == 0xFFFF {
                return 0x40;
            }
            nes.wram[addr as usize]
        }
        fn write(nes: &mut Nes, addr: u16, value: u8) {
            nes.wram[addr as usize] = value
        }
    }

    #[test]
    fn system_functions() {
        // BRK
        {
            let mut nes = Nes::new();
            nes.cpu.pc = 0x020F;
            nes.wram[0x020F] = 0x00;
            nes.cpu.p = Status::V | Status::D | Status::C;
            nes.cpu.s = 0xBF;
            // $FFFE/F = 0x23/0x40 in CpuBusMockForBRK

            Emu::cpu_step::<CpuBusMockForBRK, CpuTickMock>(&mut nes);
            assert_eq!(nes.cpu.pc, 0x4023);
            assert_eq!(nes.cpu_cycles, 7);
            assert_eq!(nes.cpu.s, 0xBC);
            assert_eq!(
                nes.cpu.p,
                Status::V | Status::D | Status::C | Status::INSTRUCTION_B
            );
        }
        // RTI
        {
            let mut nes = Nes::new();
            nes.cpu.pc = 0x020F;
            nes.wram[0x020F] = 0x40;
            nes.cpu.p = Status::V | Status::D | Status::C | Status::I;

            nes.cpu.s = 0xBC;
            nes.wram[0x00BD] = (Status::N | Status::Z).bits();
            nes.wram[0x00BE] = 0x11;
            nes.wram[0x00BF] = 0x02;

            Emu::cpu_step::<CpuBusMock, CpuTickMock>(&mut nes);
            assert_eq!(nes.cpu.s, 0xBF);
            assert_eq!(nes.cpu.p, Status::N | Status::Z);
            assert_eq!(nes.cpu.pc, 0x0211);
            assert_eq!(nes.cpu_cycles, 6);
        }
    }
}
