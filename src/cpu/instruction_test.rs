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
