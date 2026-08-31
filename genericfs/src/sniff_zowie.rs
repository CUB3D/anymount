//! ZOWIEBOX

use memmap2::Mmap;

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{gen_item::GenItem, generic_fs::GenFS};

pub struct ZowieboxF {}

impl GenFSProps for ZowieboxF {
    const FORMAT_NAME: &'static str = "zowiebox";
}

impl GenFS for ZowieboxF {
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
        Ok(f.get(..8) == Some(b"ZOWIEBOX"))
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        Ok(None)
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }
}
