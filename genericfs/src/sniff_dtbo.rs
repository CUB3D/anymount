//! Android DTBO

use memmap2::Mmap;

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{gen_item::GenItem, generic_fs::GenFS};

pub struct DtboF {}

impl GenFSProps for DtboF {
    const FORMAT_NAME: &'static str = "dtbo";
}

impl GenFS for DtboF {
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
        Ok(f.get(..4) == Some([0xd7, 0xb7, 0xab, 0x1e].as_slice()))
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        Ok(None)
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }
}
