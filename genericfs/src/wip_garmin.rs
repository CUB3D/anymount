//! Garmin firmware file, WIP

use parse::{le_u16, ne_u8, take};

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{gen_item::GenItem, generic_fs::GenFS};

pub struct GarminF {}

impl GenFSProps for GarminF {
    const FORMAT_NAME: &'static str = "garmin";
}

impl GenFS for GarminF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        //TODO: garmin partition?
        let mut tmp = &f.mmap[0xE00..];

        for _ in 0..30 {
            let (asdf, _) = take(tmp, 0x200)?;
            let (i, _x) = ne_u8(tmp)?;
            let (i, n) = take(i, 11)?;
            let n = String::from_utf8_lossy(n).to_string();
            println!("{:?}", n);

            let (i, _id) = le_u16(i)?;
            let (_i, _sz) = le_u16(i)?;

            tmp = asdf;
        }

        Ok(Self {})
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        Ok(None)
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }
}
