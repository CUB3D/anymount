//! Shannon modem fw file, sniff only
//! https://github.com/grant-h/ShannonBaseband/blob/master/reversing/ghidra/ShannonLoader/src/main/java/de/hernan/TOCSectionHeader.java

use memmap2::Mmap;

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{gen_item::GenItem, generic_fs::GenFS};

pub struct ShannonF {}

impl GenFSProps for ShannonF {
    const FORMAT_NAME: &'static str = "shannon_modem";
}

impl GenFS for ShannonF {
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
        if f.get(..12)
            == Some(&[
                0x54u8, 0x4F, 0x43, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ])
        {
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
