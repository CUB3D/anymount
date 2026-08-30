//! MTK HLBR, found in modem-bundle.img in pixel 11 pro modem
//!
use memmap2::Mmap;

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{gen_item::GenItem, generic_fs::GenFS};

pub struct MtkHblrF {}

impl GenFSProps for MtkHblrF {
    const FORMAT_NAME: &'static str = "mtk_hblr";
}

impl GenFS for MtkHblrF {
    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        if f.get(..8) == Some(b"HBLR") {
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
