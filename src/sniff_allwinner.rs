//! Allwinner A10 / ZOWIEBOX
//! https://github.com/Ithamar/awutils/blob/master/awimage.c
//!
use memmap2::Mmap;

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{gen_item::GenItem, generic_fs::GenFS};

pub struct AllwinnerA10F {}

impl GenFSProps for AllwinnerA10F {
    const FORMAT_NAME: &'static str = "allwinner_a10";
}

impl GenFS for AllwinnerA10F {
    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        if f.get(..8) == Some(b"IMAGEWTY") {
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

    fn sniff_only(&self) -> bool {
        true
    }
}
