#[macro_use]
extern crate bitflags;

extern crate anyhow;

#[cfg(test)]
#[macro_use]
extern crate assert_matches;

mod cpu;
mod nes;
mod rom;

pub struct Emu {}

impl Emu {
    #[allow(dead_code)]
    fn run_frame() {}
}
