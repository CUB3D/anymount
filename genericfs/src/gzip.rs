use std::io::Read;

use flate2::read::GzDecoder;
use memmap2::Mmap;

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};

pub struct GzipF {
    pub mmap: Mmap,
    pub idx: usize,
}

impl GenFSProps for GzipF {
    const FORMAT_NAME: &'static str = "gz";
}

impl GenFS for GzipF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            mmap: f.owned_map(),
            idx: 0,
        })
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        Ok(f.get(..2) == Some(&[0x1f, 0x8b]))
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        if self.idx == 1 {
            return Ok(None);
        }

        let mut f = GzDecoder::new(self.mmap.as_ref());

        let mut d = Vec::new();
        f.read_to_end(&mut d)?;
        self.idx += 1;
        Ok(Some(Box::new(BufGenItm {
            name: "_dec".to_string(),
            data: d,
            pos: 0,
        })))
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }

    fn single_output(&self) -> bool {
        true
    }
}
