//! Qualcomm FBPT sniff

use memmap2::Mmap;
use parse::le_u32;

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{gen_item::GenItem, generic_fs::GenFS};

pub struct FbptF {}

impl GenFSProps for FbptF {
    const FORMAT_NAME: &'static str = "fbpt";
}

impl GenFS for FbptF {
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
        Ok(f.get(..4) == Some(b"FBPT"))
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        Ok(None)
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }
}