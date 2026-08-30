use std::io::Read;

use memmap2::Mmap;

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};

pub struct Lz4F {
    pub f: Mmap,
    pub idx: usize,
}

impl GenFSProps for Lz4F {
    const FORMAT_NAME: &'static str = "lz4";
}

impl GenFS for Lz4F {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            f: f.owned_map(),
            idx: 0,
        })
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        Ok(f.get(..4) == Some(0x184D2204_u32.to_le_bytes().as_slice()))
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        if self.idx == 1 {
            return Ok(None);
        }

        let mut f = lz4_flex::frame::FrameDecoder::new(&self.f[..]);
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
