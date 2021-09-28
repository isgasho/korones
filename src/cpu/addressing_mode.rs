use super::*;

pub(super) fn get_operand<B: CpuBus, T: CpuTick>(
    nes: &mut Nes,
    addressing_mode: AddressingMode,
) -> u16 {
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
            let v = read_on_indirect::<B, T>(nes, m.wrapping_add(nes.cpu.x) as u16);
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

#[cfg(test)]
mod test {
    use super::test_mock::*;
    use super::*;

    #[test]
    fn implicit() {
        let mut nes = Nes::new();

        let v = super::get_operand::<CpuBusMock, CpuTickMock>(&mut nes, AddressingMode::Implicit);
        assert_eq!(v, 0);
        assert_eq!(nes.cpu_cycles, 0);
    }

    #[test]
    fn accumulator() {
        let mut nes = Nes::new();
        nes.cpu.a = 0xFB;

        let v =
            super::get_operand::<CpuBusMock, CpuTickMock>(&mut nes, AddressingMode::Accumulator);
        assert_eq!(v, 0xFB);
        assert_eq!(nes.cpu_cycles, 0);
    }

    #[test]
    fn immediate() {
        let mut nes = Nes::new();
        nes.cpu.pc = 0x8234;
        let v = super::get_operand::<CpuBusMock, CpuTickMock>(&mut nes, AddressingMode::Immediate);
        assert_eq!(v, 0x8234);
        assert_eq!(nes.cpu_cycles, 0);
    }

    #[test]
    fn zero_page() {
        let mut nes = Nes::new();
        nes.cpu.pc = 0x0414;
        nes.wram[0x0414] = 0x91;

        let v = super::get_operand::<CpuBusMock, CpuTickMock>(&mut nes, AddressingMode::ZeroPage);
        assert_eq!(v, 0x91);
        assert_eq!(nes.cpu_cycles, 1);
    }

    #[test]
    fn zero_page_x() {
        let mut nes = Nes::new();
        nes.cpu.pc = 0x0100;
        nes.wram[0x0100] = 0x80;
        nes.cpu.x = 0x93;

        let v = super::get_operand::<CpuBusMock, CpuTickMock>(&mut nes, AddressingMode::ZeroPageX);
        assert_eq!(v, 0x13);
        assert_eq!(nes.cpu_cycles, 1);
    }

    #[test]
    fn zero_page_y() {
        let mut nes = Nes::new();
        nes.cpu.pc = 0x0423;
        nes.wram[0x0423] = 0x36;
        nes.cpu.y = 0xF1;

        let v = super::get_operand::<CpuBusMock, CpuTickMock>(&mut nes, AddressingMode::ZeroPageY);
        assert_eq!(v, 0x27);
        assert_eq!(nes.cpu_cycles, 1);
    }

    #[test]
    fn absolute() {
        let mut nes = Nes::new();
        nes.cpu.pc = 0x0423;
        nes.wram[0x0423] = 0x36;
        nes.wram[0x0424] = 0xF0;

        let v = super::get_operand::<CpuBusMock, CpuTickMock>(&mut nes, AddressingMode::Absolute);
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

            let v = super::get_operand::<CpuBusMock, CpuTickMock>(
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

            let v = super::get_operand::<CpuBusMock, CpuTickMock>(
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

        let v = super::get_operand::<CpuBusMock, CpuTickMock>(&mut nes, AddressingMode::Relative);
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

        let v = super::get_operand::<CpuBusMock, CpuTickMock>(&mut nes, AddressingMode::Indirect);
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

        let v = super::get_operand::<CpuBusMock, CpuTickMock>(
            &mut nes,
            AddressingMode::IndexedIndirect,
        );
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

            let v = super::get_operand::<CpuBusMock, CpuTickMock>(
                &mut nes,
                AddressingMode::IndirectIndexed,
            );
            assert_eq!(v, expected_operand, "{}", name);
            assert_eq!(nes.cpu_cycles, expected_cycles, "{}", name);
        }
    }
}
