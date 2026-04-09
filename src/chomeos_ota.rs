//! ChromeOS OTA files

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{gen_item::GenItem, generic_fs::GenFS};
use memmap2::Mmap;
use tracing::error;

///See https://github.com/sebanc/chromeos-ota-extract/blob/master/extract_android_ota_payload.py
pub struct ChromeosOTAF {}

impl GenFSProps for ChromeosOTAF {
    const FORMAT_NAME: &'static str = "ChromeOS OTA";
}

impl GenFS for ChromeosOTAF {
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
        let hdr = f.get(..4);

        if hdr != Some(b"CrAU") {
            return Ok(false);
        }

        Ok(true)
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        error!("CrAU EXTRACT NOT SUPPORTED");
        Ok(None)
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }
}
