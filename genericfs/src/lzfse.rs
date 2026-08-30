//! LZFSE decompression

use memmap2::Mmap;

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};

pub struct LzfseF {
    pub f: Mmap,
    pub idx: usize,
}

impl GenFSProps for LzfseF {
    const FORMAT_NAME: &'static str = "lzfse";
}

impl GenFS for LzfseF {
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
        Ok(f.get(..3) == Some(b"bvx"))
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        if self.idx == 1 {
            return Ok(None);
        }

        let mut d = Vec::new();
        lzfse_rust::decode_bytes(&self.f[..], &mut d)?;
        self.idx += 1;
        Ok(Some(Box::new(BufGenItm::new("_dec", d))))
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }

    fn single_output(&self) -> bool {
        true
    }
}