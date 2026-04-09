//! fbpack ("FastBoot PacK") - used by Pixel modems
//! https://github.com/LineageOS/scripts/tree/main/fbpacktool - qualcomm modem pixel 5 mar 2023

use memmap2::Mmap;
use parse::le_u32;

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{gen_item::GenItem, generic_fs::GenFS};

pub struct FbpackF {}

impl GenFSProps for FbpackF {
    const FORMAT_NAME: &'static str = "fbpack";
}

impl GenFS for FbpackF {
    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        let (i, magic) = le_u32(f)?;
        let (_i, version) = le_u32(i)?;

        // FBPK
        if magic == 0x4b504246 && (version == 1 || version == 2) {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn try_open_internal(_f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {})
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        Ok(None)
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }
}
