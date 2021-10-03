use std::fmt;
use std::io::BufReader;
use std::io::Read;

use anyhow::Result;

use crate::nes::Mirroring;

#[allow(dead_code)]
pub(crate) fn parse(rom: &[u8]) -> Result<(Header, Vec<u8>)> {
    let mut cur = BufReader::new(rom);

    // validate magic number
    let _ = {
        let mut buf = [0; 4];
        cur.read_exact(&mut buf)?;
        if buf != [0x4E, 0x45, 0x53, 0x1A] {
            Err(ParseError {
                msg: "invalid magic number".to_string(),
            })
        } else {
            Ok(())
        }
    }?;

    let prg_rom_size = {
        let mut buf = [0; 1];
        cur.read_exact(&mut buf)?;
        buf[0]
    };
    let chr_rom_size = {
        let mut buf = [0; 1];
        cur.read_exact(&mut buf)?;
        buf[0]
    };
    // flag 6
    let mirroring = {
        let mut buf = [0; 1];
        cur.read_exact(&mut buf)?;
        let b = buf[0];
        if b & 1 == 0 {
            Mirroring::Horizontal
        } else {
            Mirroring::Vertical
        }
    };

    // skip flag 7, 8, 9, 10
    {
        let mut buf = [0; 4];
        cur.read_exact(&mut buf)?;
    }

    // validate unused padding
    {
        let mut buf = [0; 4];
        cur.read_exact(&mut buf)?;
        if buf != [0; 4] {
            Err(ParseError {
                msg: "invalid padding".to_string(),
            })
        } else {
            Ok(())
        }
    }?;

    let mut buf = Vec::new();
    cur.read_to_end(&mut buf)?;

    Ok((
        Header {
            prg_rom_size,
            chr_rom_size,
            mirroring,
        },
        buf,
    ))
}

#[derive(Debug)]
pub(crate) struct Header {
    prg_rom_size: u8,
    chr_rom_size: u8,
    mirroring: Mirroring,
}

#[derive(Clone, Debug)]
pub(crate) struct ParseError {
    msg: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "iNES file parse error: {}", self.msg)
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod test {
    use super::*;

    use std::fs::File;
    use std::path::Path;

    #[test]
    fn test_parse() {
        let root = env!("CARGO_MANIFEST_DIR");
        let path = Path::new(root).join("roms/nestest.nes");

        let mut f = File::open(path).unwrap();
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).unwrap();

        let result = parse(&buf);

        assert_matches!(
            result,
            Ok((
                Header {
                    prg_rom_size: 1,
                    chr_rom_size: 1,
                    mirroring: Mirroring::Horizontal,
                },
                _
            ))
        )
    }
}
