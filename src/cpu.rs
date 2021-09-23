use crate::nes::Nes;
use crate::Emu;

#[derive(Debug, Default)]
pub struct Cpu {
    a: u8,
    x: u8,
    y: u8,
    pc: u16,
}

pub(crate) trait CpuBus {
    fn read(nes: &mut Nes, addr: u16) -> u8;
    fn write(nes: &mut Nes, addr: u16, value: u8);
}

pub(crate) trait CpuTick {
    fn tick(nes: &mut Nes);
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

impl Emu {
    fn cpu_step<B: CpuBus, T: CpuTick>(nes: &mut Nes) {
        let opcode = read::<B, T>(nes, nes.cpu.pc);
        nes.cpu.pc = nes.cpu.pc.wrapping_add(1);

        let instruction = decode(opcode);
        Self::execute::<B, T>(nes, instruction);
    }

    fn execute<B: CpuBus, T: CpuTick>(nes: &mut Nes, instruction: Instruction) {
        // get operand
        let (_, addressing_mode) = &instruction;
        let operand = Self::get_operand::<B, T>(nes, *addressing_mode);

        //TODO
        match instruction {
            (Mnemonic::ADC, _) => {}
            _ => {}
        }
    }

    fn get_operand<B: CpuBus, T: CpuTick>(nes: &mut Nes, addressing_mode: AddressingMode) -> u16 {
        match addressing_mode {
            AddressingMode::Implicit => 0u16,
            AddressingMode::Accumulator => nes.cpu.a as u16,
            AddressingMode::Immediate => {
                let pc = nes.cpu.pc;
                nes.cpu.pc = nes.cpu.pc.wrapping_add(1);
                pc
            }
            AddressingMode::ZeroPage => {
                let v = read::<B, T>(nes, nes.cpu.pc);
                nes.cpu.pc = nes.cpu.pc.wrapping_add(1);
                v as u16
            }
            AddressingMode::ZeroPageX => {
                let v = (read::<B, T>(nes, nes.cpu.pc) as u16 + nes.cpu.x as u16) & 0xFF;
                nes.cpu.pc = nes.cpu.pc.wrapping_add(1);
                v as u16
            }
            AddressingMode::ZeroPageY => {
                let v = (read::<B, T>(nes, nes.cpu.pc) as u16 + nes.cpu.y as u16) & 0xFF;
                nes.cpu.pc = nes.cpu.pc.wrapping_add(1);
                v as u16
            }
            AddressingMode::Absolute => {
                let v = read_word::<B, T>(nes, nes.cpu.pc);
                nes.cpu.pc = nes.cpu.pc.wrapping_add(2);
                v as u16
            }
            AddressingMode::AbsoluteX { oops } => {
                let v = read_word::<B, T>(nes, nes.cpu.pc);
                nes.cpu.pc = nes.cpu.pc.wrapping_add(2);
                if oops {
                    if page_crossed(nes.cpu.x as u16, v) {
                        T::tick(nes);
                    }
                } else {
                    T::tick(nes);
                }
                (v as u16).wrapping_add(nes.cpu.x as u16)
            }
            AddressingMode::AbsoluteY { oops } => {
                let v = read_word::<B, T>(nes, nes.cpu.pc);
                nes.cpu.pc = nes.cpu.pc.wrapping_add(2);
                if oops {
                    if page_crossed(nes.cpu.y as u16, v) {
                        T::tick(nes);
                    }
                } else {
                    T::tick(nes);
                }
                (v as u16).wrapping_add(nes.cpu.y as u16)
            }
            AddressingMode::Relative => {
                let v = read::<B, T>(nes, nes.cpu.pc);
                nes.cpu.pc = nes.cpu.pc.wrapping_add(1);
                v as u16
            }
            AddressingMode::Indirect => {
                let m = read_word::<B, T>(nes, nes.cpu.pc);
                let v = read_on_indirect::<B, T>(nes, m);
                nes.cpu.pc = nes.cpu.pc.wrapping_add(2);
                v
            }
            AddressingMode::IndexedIndirect => {
                let m = read::<B, T>(nes, nes.cpu.pc);
                let v = read_on_indirect::<B, T>(nes, (m.wrapping_add(nes.cpu.x) & 0xFF) as u16);
                nes.cpu.pc = nes.cpu.pc.wrapping_add(1);
                T::tick(nes);
                v
            }
            AddressingMode::IndirectIndexed => {
                let m = read::<B, T>(nes, nes.cpu.pc);
                let n = read_on_indirect::<B, T>(nes, m as u16);
                let v = n.wrapping_add(nes.cpu.y as u16);
                nes.cpu.pc = nes.cpu.pc.wrapping_add(1);
                if page_crossed(nes.cpu.y as u16, n) {
                    T::tick(nes);
                }
                v
            }
        }
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
        0xB6 => (Mnemonic::LDX, AddressingMode::ZeroPageX),
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

        0x6A => (Mnemonic::ROL, AddressingMode::Accumulator),
        0x66 => (Mnemonic::ROL, AddressingMode::ZeroPage),
        0x76 => (Mnemonic::ROL, AddressingMode::ZeroPageX),
        0x6E => (Mnemonic::ROL, AddressingMode::Absolute),
        0x7E => (Mnemonic::ROL, AddressingMode::AbsoluteX { oops: false }),

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
        0x96 => (Mnemonic::STX, AddressingMode::ZeroPageX),
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
mod addressing_mode_tests {
    use super::*;

    struct CpuTickMock {}
    impl CpuTick for CpuTickMock {
        fn tick(nes: &mut Nes) {
            nes.cpu_cycles = nes.cpu_cycles.wrapping_add(1);
        }
    }

    struct CpuBusMock {}
    impl CpuBus for CpuBusMock {
        fn read(nes: &mut Nes, addr: u16) -> u8 {
            nes.wram[addr as usize]
        }
        fn write(nes: &mut Nes, addr: u16, value: u8) {
            nes.wram[addr as usize] = value
        }
    }

    #[test]
    fn implicit() {
        let mut nes = Nes::new();

        let v = Emu::get_operand::<CpuBusMock, CpuTickMock>(&mut nes, AddressingMode::Implicit);
        assert_eq!(v, 0);
        assert_eq!(nes.cpu_cycles, 0);
    }

    #[test]
    fn accumulator() {
        let mut nes = Nes::new();
        nes.cpu.a = 0xFB;

        let v = Emu::get_operand::<CpuBusMock, CpuTickMock>(&mut nes, AddressingMode::Accumulator);
        assert_eq!(v, 0xFB);
        assert_eq!(nes.cpu_cycles, 0);
    }

    #[test]
    fn immediate() {
        let mut nes = Nes::new();
        nes.cpu.pc = 0x8234;
        let v = Emu::get_operand::<CpuBusMock, CpuTickMock>(&mut nes, AddressingMode::Immediate);
        assert_eq!(v, 0x8234);
        assert_eq!(nes.cpu_cycles, 0);
    }

    #[test]
    fn zero_page() {
        let mut nes = Nes::new();
        nes.cpu.pc = 0x0414;
        nes.wram[0x0414] = 0x91;

        let v = Emu::get_operand::<CpuBusMock, CpuTickMock>(&mut nes, AddressingMode::ZeroPage);
        assert_eq!(v, 0x91);
        assert_eq!(nes.cpu_cycles, 1);
    }

    #[test]
    fn zero_page_x() {
        let mut nes = Nes::new();
        nes.cpu.pc = 0x0100;
        nes.wram[0x0100] = 0x80;
        nes.cpu.x = 0x93;

        let v = Emu::get_operand::<CpuBusMock, CpuTickMock>(&mut nes, AddressingMode::ZeroPageX);
        assert_eq!(v, 0x13);
        assert_eq!(nes.cpu_cycles, 1);
    }

    #[test]
    fn zero_page_y() {
        let mut nes = Nes::new();
        nes.cpu.pc = 0x0423;
        nes.wram[0x0423] = 0x36;
        nes.cpu.y = 0xF1;

        let v = Emu::get_operand::<CpuBusMock, CpuTickMock>(&mut nes, AddressingMode::ZeroPageY);
        assert_eq!(v, 0x27);
        assert_eq!(nes.cpu_cycles, 1);
    }

    #[test]
    fn absolute() {
        let mut nes = Nes::new();
        nes.cpu.pc = 0x0423;
        nes.wram[0x0423] = 0x36;
        nes.wram[0x0424] = 0xF0;

        let v = Emu::get_operand::<CpuBusMock, CpuTickMock>(&mut nes, AddressingMode::Absolute);
        assert_eq!(v, 0xF036);
        assert_eq!(nes.cpu_cycles, 2);
    }

    #[test]
    fn absolute_x() {
        #[rustfmt::skip]
        let cases = [
            ("no oops",               false, 0x31, 0xF067, 3),
            ("oops/not page crossed", true,  0x31, 0xF067, 2),
            ("oops/page crossed",     true,  0xF0, 0xF126, 3),
        ];

        for (name, oops, x, expected_operand, expected_cycles) in cases {
            let mut nes = Nes::new();
            nes.cpu.pc = 0x0423;
            nes.wram[0x0423] = 0x36;
            nes.wram[0x0424] = 0xF0;

            nes.cpu.x = x;

            let v = Emu::get_operand::<CpuBusMock, CpuTickMock>(
                &mut nes,
                AddressingMode::AbsoluteX { oops },
            );
            assert_eq!(v, expected_operand, "{}", name);
            assert_eq!(nes.cpu_cycles, expected_cycles, "{}", name);
        }
    }

    #[test]
    fn absolute_y() {
        #[rustfmt::skip]
        let cases = [
            ("no oops",               false, 0x31, 0xF067, 3),
            ("oops/not page crossed", true,  0x31, 0xF067, 2),
            ("oops/page crossed",     true,  0xF0, 0xF126, 3),
        ];

        for (name, oops, y, expected_operand, expected_cycles) in cases {
            let mut nes = Nes::new();
            nes.cpu.pc = 0x0423;
            nes.wram[0x0423] = 0x36;
            nes.wram[0x0424] = 0xF0;

            nes.cpu.y = y;

            let v = Emu::get_operand::<CpuBusMock, CpuTickMock>(
                &mut nes,
                AddressingMode::AbsoluteY { oops },
            );
            assert_eq!(v, expected_operand, "{}", name);
            assert_eq!(nes.cpu_cycles, expected_cycles, "{}", name);
        }
    }

    #[test]
    fn relative() {
        let mut nes = Nes::new();
        nes.cpu.pc = 0x0414;
        nes.wram[0x0414] = 0x91;

        let v = Emu::get_operand::<CpuBusMock, CpuTickMock>(&mut nes, AddressingMode::Relative);
        assert_eq!(v, 0x91);
        assert_eq!(nes.cpu_cycles, 1);
    }

    #[test]
    fn indirect() {
        let mut nes = Nes::new();
        nes.cpu.pc = 0x020F;
        nes.wram[0x020F] = 0x10;
        nes.wram[0x0210] = 0x03;
        nes.wram[0x0310] = 0x9F;

        let v = Emu::get_operand::<CpuBusMock, CpuTickMock>(&mut nes, AddressingMode::Indirect);
        assert_eq!(v, 0x9F);
        assert_eq!(nes.cpu_cycles, 4);
    }

    #[test]
    fn indexed_indirect() {
        let mut nes = Nes::new();
        nes.cpu.pc = 0x020F;
        nes.wram[0x020F] = 0xF0;
        nes.cpu.x = 0x95;
        nes.wram[0x0085] = 0x12;
        nes.wram[0x0086] = 0x90;

        let v =
            Emu::get_operand::<CpuBusMock, CpuTickMock>(&mut nes, AddressingMode::IndexedIndirect);
        assert_eq!(v, 0x9012);
        assert_eq!(nes.cpu_cycles, 4);
    }

    #[test]
    fn indirect_indexed() {
        #[rustfmt::skip]
        let cases = [
            ("not page crossed", 0x83, 0x9095, 3),
            ("page crossed",     0xF3, 0x9105, 4),
        ];

        for (name, y, expected_operand, expected_cycles) in cases {
            let mut nes = Nes::new();
            nes.cpu.pc = 0x020F;
            nes.wram[0x020F] = 0xF0;
            nes.wram[0x00F0] = 0x12;
            nes.wram[0x00F1] = 0x90;
            nes.cpu.y = y;

            let v = Emu::get_operand::<CpuBusMock, CpuTickMock>(
                &mut nes,
                AddressingMode::IndirectIndexed,
            );
            assert_eq!(v, expected_operand, "{}", name);
            assert_eq!(nes.cpu_cycles, expected_cycles, "{}", name);
        }
    }
}
