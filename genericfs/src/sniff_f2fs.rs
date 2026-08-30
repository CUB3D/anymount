//! f2fs, sniff only

use memmap2::Mmap;

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{gen_item::GenItem, generic_fs::GenFS};

pub struct F2fsF {}

impl GenFSProps for F2fsF {
    const FORMAT_NAME: &'static str = "f2fs";
}

impl GenFS for F2fsF {
    fn try_open_internal(_f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {})
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        if f.get(0x400..0x404) == Some(&[0x10u8, 0x20, 0xf5, 0xf2]) {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        Ok(None)
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }

    fn sniff_only(&self) -> bool {
        true
    }
}
