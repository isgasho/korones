use super::*;

pub(super) fn execute<B: CpuBus, T: CpuTick>(
    nes: &mut Nes,
    instruction: Instruction,
    operand: u16,
) {
    match instruction {
        (Mnemonic::LDA, _) => {
            nes.cpu.a = read::<B, T>(nes, operand);
            nes.cpu.p.set_zn(nes.cpu.a);
        }
        (Mnemonic::LDX, _) => {
            nes.cpu.x = read::<B, T>(nes, operand);
            nes.cpu.p.set_zn(nes.cpu.x);
        }
        (Mnemonic::LDY, _) => {
            nes.cpu.y = read::<B, T>(nes, operand);
            nes.cpu.p.set_zn(nes.cpu.y);
        }
        (Mnemonic::STA, _) => {
            write::<B, T>(nes, operand, nes.cpu.a);
        }
        (Mnemonic::STX, _) => {
            write::<B, T>(nes, operand, nes.cpu.x);
        }
        (Mnemonic::STY, _) => {
            write::<B, T>(nes, operand, nes.cpu.y);
        }

        (Mnemonic::TAX, _) => {
            nes.cpu.x = nes.cpu.a;
            nes.cpu.p.set_zn(nes.cpu.x);
            T::tick(nes);
        }
        (Mnemonic::TAY, _) => {
            nes.cpu.y = nes.cpu.a;
            nes.cpu.p.set_zn(nes.cpu.y);
            T::tick(nes);
        }
        (Mnemonic::TXA, _) => {
            nes.cpu.a = nes.cpu.x;
            nes.cpu.p.set_zn(nes.cpu.a);
            T::tick(nes);
        }
        (Mnemonic::TYA, _) => {
            nes.cpu.a = nes.cpu.y;
            nes.cpu.p.set_zn(nes.cpu.a);
            T::tick(nes);
        }

        (Mnemonic::TSX, _) => {
            nes.cpu.x = nes.cpu.s;
            nes.cpu.p.set_zn(nes.cpu.x);
            T::tick(nes);
        }
        (Mnemonic::TXS, _) => {
            nes.cpu.s = nes.cpu.x;
            T::tick(nes);
        }
        (Mnemonic::PHA, _) => {
            push_stack::<B, T>(nes, nes.cpu.a);
            T::tick(nes);
        }
        (Mnemonic::PHP, _) => {
            let p = (nes.cpu.p | Status::INSTRUCTION_B).bits();
            push_stack::<B, T>(nes, p);
            T::tick(nes);
        }
        (Mnemonic::PLA, _) => {
            nes.cpu.a = pull_stack::<B, T>(nes);
            nes.cpu.p.set_zn(nes.cpu.a);
            T::tick(nes);
        }
        (Mnemonic::PLP, _) => {
            let v = pull_stack::<B, T>(nes);
            nes.cpu.p = unsafe { Status::from_bits_unchecked(v) & !Status::INSTRUCTION_B };
            T::tick_n(nes, 2);
        }

        (Mnemonic::AND, _) => {
            nes.cpu.a &= read::<B, T>(nes, operand);
            nes.cpu.p.set_zn(nes.cpu.a);
        }
        (Mnemonic::EOR, _) => {
            nes.cpu.a ^= read::<B, T>(nes, operand);
            nes.cpu.p.set_zn(nes.cpu.a);
        }
        (Mnemonic::ORA, _) => {
            nes.cpu.a |= read::<B, T>(nes, operand);
            nes.cpu.p.set_zn(nes.cpu.a);
        }
        (Mnemonic::BIT, _) => {
            let b = nes.cpu.a & read::<B, T>(nes, operand);
            nes.cpu.p.set_zn(b);
            nes.cpu.p.set(Status::V, b & 0x40 == 0x40);
        }

        (Mnemonic::ADC, _) => {
            let m = read::<B, T>(nes, operand);
            let mut r = nes.cpu.a.wrapping_add(m);

            if nes.cpu.p.contains(Status::C) {
                r = r.wrapping_add(1);
            }

            let a7 = nes.cpu.a >> 7 & 1;
            let m7 = m >> 7 & 1;
            let c6 = a7 ^ m7 ^ (r >> 7 & 1);
            let c7 = (a7 & m7) | (a7 & c6) | (m7 & c6);
            nes.cpu.p.set(Status::C, c7 == 1);
            nes.cpu.p.set(Status::V, c6 ^ c7 == 1);

            nes.cpu.a = r;
            nes.cpu.p.set_zn(nes.cpu.a);
        }
        (Mnemonic::SBC, _) => {
            let m = read::<B, T>(nes, operand);
            let mut r = nes.cpu.a.wrapping_sub(m);

            if nes.cpu.p.contains(Status::C) {
                r = r.wrapping_add(1);
            }

            let a7 = nes.cpu.a >> 7 & 1;
            let m7 = m >> 7 & 1;
            let c6 = a7 ^ m7 ^ (r >> 7 & 1);
            let c7 = (a7 & m7) | (a7 & c6) | (m7 & c6);
            nes.cpu.p.set(Status::C, c7 == 1);
            nes.cpu.p.set(Status::V, c6 ^ c7 == 1);

            nes.cpu.a = r;
            nes.cpu.p.set_zn(nes.cpu.a);
        }
        (Mnemonic::CMP, _) => {
            let r = nes.cpu.a as i16 - read::<B, T>(nes, operand) as i16;
            nes.cpu.p.set_zn(r as u8);
            nes.cpu.p.set(Status::C, 0 < r);
        }
        (Mnemonic::CPX, _) => {
            let r = nes.cpu.x as i16 - read::<B, T>(nes, operand) as i16;
            nes.cpu.p.set_zn(r as u8);
            nes.cpu.p.set(Status::C, 0 < r);
        }
        (Mnemonic::CPY, _) => {
            let r = nes.cpu.y as i16 - read::<B, T>(nes, operand) as i16;
            nes.cpu.p.set_zn(r as u8);
            nes.cpu.p.set(Status::C, 0 < r);
        }

        (Mnemonic::INC, _) => {
            let m = read::<B, T>(nes, operand);
            let r = m.wrapping_add(1);
            write::<B, T>(nes, operand, r);
            nes.cpu.p.set_zn(r);
            T::tick(nes);
        }
        (Mnemonic::INX, _) => {
            nes.cpu.x = nes.cpu.x.wrapping_add(1);
            nes.cpu.p.set_zn(nes.cpu.x);
            T::tick(nes);
        }
        (Mnemonic::INY, _) => {
            nes.cpu.y = nes.cpu.y.wrapping_add(1);
            nes.cpu.p.set_zn(nes.cpu.y);
            T::tick(nes);
        }
        (Mnemonic::DEC, _) => {
            let m = read::<B, T>(nes, operand);
            let r = m.wrapping_sub(1);
            write::<B, T>(nes, operand, r);
            nes.cpu.p.set_zn(r);
            T::tick(nes);
        }
        (Mnemonic::DEX, _) => {
            nes.cpu.x = nes.cpu.x.wrapping_sub(1);
            nes.cpu.p.set_zn(nes.cpu.x);
            T::tick(nes);
        }
        (Mnemonic::DEY, _) => {
            nes.cpu.y = nes.cpu.y.wrapping_sub(1);
            nes.cpu.p.set_zn(nes.cpu.y);
            T::tick(nes);
        }

        (Mnemonic::ASL, AddressingMode::Accumulator) => {
            nes.cpu.p.set(Status::C, nes.cpu.a & 0x80 == 0x80);
            nes.cpu.a <<= 1;
            nes.cpu.p.set_zn(nes.cpu.a);
            T::tick(nes);
        }
        (Mnemonic::ASL, _) => {
            let mut m = read::<B, T>(nes, operand);
            m <<= 1;
            nes.cpu.p.set_zn(m);
            write::<B, T>(nes, operand, m);
            T::tick(nes);
        }
        (Mnemonic::LSR, AddressingMode::Accumulator) => {
            nes.cpu.p.set(Status::C, nes.cpu.a & 0x80 == 0x80);
            nes.cpu.a >>= 1;
            nes.cpu.p.set_zn(nes.cpu.a);
            T::tick(nes);
        }
        (Mnemonic::LSR, _) => {
            let mut m = read::<B, T>(nes, operand);
            m >>= 1;
            nes.cpu.p.set_zn(m);
            write::<B, T>(nes, operand, m);
            T::tick(nes);
        }
        (Mnemonic::ROL, AddressingMode::Accumulator) => {
            let c = nes.cpu.a & 0x80;
            nes.cpu.a <<= 1;
            if nes.cpu.p.contains(Status::C) {
                nes.cpu.a |= 1;
            }
            nes.cpu.p.set(Status::C, c == 0x80);
            nes.cpu.p.set_zn(nes.cpu.a);
            T::tick(nes);
        }
        (Mnemonic::ROL, _) => {
            let mut m = read::<B, T>(nes, operand);
            let c = m & 0x80;
            m <<= 1;
            if nes.cpu.p.contains(Status::C) {
                m |= 1;
            }
            nes.cpu.p.set(Status::C, c == 0x80);
            nes.cpu.p.set_zn(m);
            write::<B, T>(nes, operand, m);
            T::tick(nes);
        }
        (Mnemonic::ROR, AddressingMode::Accumulator) => {
            let c = nes.cpu.a & 1;
            nes.cpu.a >>= 1;
            if nes.cpu.p.contains(Status::C) {
                nes.cpu.a |= 0x80;
            }
            nes.cpu.p.set(Status::C, c == 1);
            nes.cpu.p.set_zn(nes.cpu.a);
            T::tick(nes);
        }
        (Mnemonic::ROR, _) => {
            let mut m = read::<B, T>(nes, operand);
            let c = m & 1;
            m >>= 1;
            if nes.cpu.p.contains(Status::C) {
                m |= 0x80;
            }
            nes.cpu.p.set(Status::C, c == 1);
            nes.cpu.p.set_zn(m);
            write::<B, T>(nes, operand, m);
            T::tick(nes);
        }

        (Mnemonic::JMP, _) => {
            nes.cpu.pc = operand;
        }
        (Mnemonic::JSR, _) => {
            let rtn = nes.cpu.pc.wrapping_sub(1);
            push_stack_word::<B, T>(nes, rtn);
            nes.cpu.pc = operand;
            T::tick(nes);
        }
        (Mnemonic::RTS, _) => {
            nes.cpu.pc = pull_stack_word::<B, T>(nes);
            T::tick_n(nes, 3);
        }

        (Mnemonic::BCC, _) => {
            if !nes.cpu.p.contains(Status::C) {
                branch::<B, T>(nes, operand);
            }
        }
        (Mnemonic::BCS, _) => {
            if nes.cpu.p.contains(Status::C) {
                branch::<B, T>(nes, operand);
            }
        }
        (Mnemonic::BEQ, _) => {
            if nes.cpu.p.contains(Status::Z) {
                branch::<B, T>(nes, operand);
            }
        }
        (Mnemonic::BMI, _) => {
            if nes.cpu.p.contains(Status::N) {
                branch::<B, T>(nes, operand);
            }
        }
        (Mnemonic::BNE, _) => {
            if !nes.cpu.p.contains(Status::Z) {
                branch::<B, T>(nes, operand);
            }
        }
        (Mnemonic::BPL, _) => {
            if !nes.cpu.p.contains(Status::N) {
                branch::<B, T>(nes, operand);
            }
        }
        (Mnemonic::BVC, _) => {
            if !nes.cpu.p.contains(Status::V) {
                branch::<B, T>(nes, operand);
            }
        }
        (Mnemonic::BVS, _) => {
            if nes.cpu.p.contains(Status::V) {
                branch::<B, T>(nes, operand);
            }
        }

        (Mnemonic::CLC, _) => {
            nes.cpu.p.remove(Status::C);
            T::tick(nes);
        }
        (Mnemonic::CLD, _) => {
            nes.cpu.p.remove(Status::D);
            T::tick(nes);
        }
        (Mnemonic::CLI, _) => {
            nes.cpu.p.remove(Status::I);
            T::tick(nes);
        }
        (Mnemonic::CLV, _) => {
            nes.cpu.p.remove(Status::V);
            T::tick(nes);
        }
        (Mnemonic::SEC, _) => {
            nes.cpu.p.insert(Status::C);
            T::tick(nes);
        }
        (Mnemonic::SED, _) => {
            nes.cpu.p.insert(Status::D);
            T::tick(nes);
        }
        (Mnemonic::SEI, _) => {
            nes.cpu.p.insert(Status::I);
            T::tick(nes);
        }

        (Mnemonic::BRK, _) => {
            push_stack_word::<B, T>(nes, nes.cpu.pc);
            nes.cpu.p.insert(Status::INSTRUCTION_B);
            push_stack::<B, T>(nes, nes.cpu.p.bits());
            nes.cpu.pc = read_word::<B, T>(nes, 0xFFFE);
            T::tick(nes);
        }
        (Mnemonic::NOP, _) => {
            T::tick(nes);
        }
        (Mnemonic::RTI, _) => {
            let p = pull_stack::<B, T>(nes);
            nes.cpu.p = unsafe { Status::from_bits_unchecked(p) & !Status::INSTRUCTION_B };
            nes.cpu.pc = pull_stack_word::<B, T>(nes);
            T::tick_n(nes, 2);
        }
        _ => unimplemented!("nop"),
    }
}

fn branch<B: CpuBus, T: CpuTick>(nes: &mut Nes, operand: u16) {
    T::tick(nes);
    if page_crossed(operand, nes.cpu.pc) {
        T::tick(nes);
    }
    nes.cpu.pc = nes.cpu.pc.wrapping_add(operand);
}

impl Status {
    fn set_zn(&mut self, v: u8) {
        self.set(Self::Z, v == 0);
        self.set(Self::N, v & 0x80 == 0x80);
    }
}
