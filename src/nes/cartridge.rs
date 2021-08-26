const NES_FILE_SIGNATURE: [u8; 4] = [0x4E, 0x45, 0x53, 0x1A];
const PRG_ROM_PAGE_SIZE: usize = 16384; // 16KB
const CHR_ROM_PAGE_SIZE: usize = 8192; // 8KB

#[derive(Debug, PartialEq)]
pub enum MirroringMode {
    Vertical,
    Horizontal,
    FourScreen,
}

pub struct Rom {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    mapper: u8,
    screen_mirroring: MirroringMode,
}

impl Rom {
    pub fn new(raw_data: &Vec<u8>) -> Result<Self, String> {
        if &raw_data[0..4] != NES_FILE_SIGNATURE {
            return Err("ROM data is not in iNES file format".to_string());
        }

        let ines_version = (raw_data[7] >> 2) & 0b11;
        if ines_version != 0 {
            return Err("NES2.0 ROM format not supported".to_string());
        }

        let is_mirroring_four_screen = raw_data[6] & 0b1000 != 0;
        let is_mirroring_vertical = raw_data[6] & 0b1 != 0;
        let screen_mirroring = match (is_mirroring_four_screen, is_mirroring_vertical) {
            (true, _) => MirroringMode::FourScreen,
            (false, true) => MirroringMode::Vertical,
            (false, false) => MirroringMode::Horizontal,
        };

        let mapper = (raw_data[7] & 0b1111_0000) | (raw_data[6] >> 4);
        let skip_trainer = raw_data[6] & 0b100 != 0;

        let prg_rom_size = raw_data[4] as usize * PRG_ROM_PAGE_SIZE;
        let chr_rom_size = raw_data[5] as usize * CHR_ROM_PAGE_SIZE;

        let prg_rom_start_pos = 16 + if skip_trainer { 512 } else { 0 };
        let chr_rom_start_pos = prg_rom_start_pos + prg_rom_size;

        Ok(Rom {
            prg_rom: raw_data[prg_rom_start_pos..(prg_rom_start_pos + prg_rom_size)].to_vec(),
            chr_rom: raw_data[chr_rom_start_pos..(chr_rom_start_pos + chr_rom_size)].to_vec(),
            mapper,
            screen_mirroring,
        })
    }

    pub fn prg_rom(&self) -> &Vec<u8> {
        &self.prg_rom
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    struct InputRomData {
        header: Vec<u8>,
        trainer: Option<Vec<u8>>,
        prg_rom: Vec<u8>,
        chr_rom: Vec<u8>,
    }

    fn create_rom(input: InputRomData) -> Vec<u8> {
        let mut result = Vec::with_capacity(
            input.header.len()
                + input.trainer.as_ref().map_or(0, |t| t.len())
                + input.prg_rom.len()
                + input.chr_rom.len(),
        );

        result.extend(&input.header);
        if let Some(t) = input.trainer {
            result.extend(t);
        }
        result.extend(&input.prg_rom);
        result.extend(&input.chr_rom);
        result
    }

    pub fn create_simple_test_rom() -> Rom {
        let test_rom = create_rom(InputRomData {
            header: vec![
                0x4E, 0x45, 0x53, 0x1A, 0x02, 0x01, 0x31, 00, 00, 00, 00, 00, 00, 00, 00, 00,
            ],
            trainer: None,
            prg_rom: vec![1; 2 * PRG_ROM_PAGE_SIZE],
            chr_rom: vec![2; 1 * CHR_ROM_PAGE_SIZE],
        });

        Rom::new(&test_rom).unwrap()
    }

    pub fn create_simple_test_rom_with_data(
        raw_prg_data: Vec<u8>,
        raw_chr_data: Option<Vec<u8>>,
    ) -> Rom {
        let mut test_rom = create_simple_test_rom();

        test_rom.prg_rom[0..raw_prg_data.len()].copy_from_slice(&raw_prg_data[..]);
        test_rom.prg_rom[(0xFFFC-0x8000) as usize] = 0x00;
        test_rom.prg_rom[(0xFFFD-0x8000) as usize] = 0x80;
        // CPU reads position 0xFFFC to get the start of the program_counter
        // The 0xFFFC-0x8000 is to set the address relative to zero
        // CPU start at address 0x8000

        if raw_chr_data.is_some() {
            let chr_data = raw_chr_data.as_ref().unwrap();
            test_rom.chr_rom[0..chr_data.len()]
                .copy_from_slice(&chr_data[..]);
        }

        test_rom
    }

    #[test]
    fn test_rom_creation() {
        let rom = create_simple_test_rom();
        assert_eq!(rom.prg_rom, vec![1; 2 * PRG_ROM_PAGE_SIZE]);
        assert_eq!(rom.chr_rom, vec![2; 1 * CHR_ROM_PAGE_SIZE]);
        assert_eq!(rom.mapper, 3);
        assert_eq!(rom.screen_mirroring, MirroringMode::Vertical);
    }

    #[test]
    fn test_rom_creation_with_trainer() {
        let raw_rom = create_rom(InputRomData {
            header: vec![
                0x4E,
                0x45,
                0x53,
                0x1A,
                0x02,
                0x01,
                0x31 | 0b100,
                00,
                00,
                00,
                00,
                00,
                00,
                00,
                00,
                00,
            ],
            trainer: Some(vec![0; 512]),
            prg_rom: vec![1; 2 * PRG_ROM_PAGE_SIZE],
            chr_rom: vec![2; 1 * CHR_ROM_PAGE_SIZE],
        });

        let rom = Rom::new(&raw_rom).unwrap();
        assert_eq!(rom.prg_rom, vec![1; 2 * PRG_ROM_PAGE_SIZE]);
        assert_eq!(rom.chr_rom, vec![2; 1 * CHR_ROM_PAGE_SIZE]);
        assert_eq!(rom.mapper, 3);
        assert_eq!(rom.screen_mirroring, MirroringMode::Vertical);
    }

    #[test]
    fn test_ines2_not_supported() {
        let test_rom = create_rom(InputRomData {
            header: vec![
                0x4E, 0x45, 0x53, 0x1A, 0x01, 0x01, 0x31, 0x8, 00, 00, 00, 00, 00, 00, 00, 00,
            ],
            trainer: None,
            prg_rom: vec![1; 1 * PRG_ROM_PAGE_SIZE],
            chr_rom: vec![2; 1 * CHR_ROM_PAGE_SIZE],
        });
        let rom = Rom::new(&test_rom);
        match rom {
            Result::Ok(_) => assert!(false, "It should not load the specified rom!"),
            Result::Err(_) => assert!(true),
        }
    }
}
