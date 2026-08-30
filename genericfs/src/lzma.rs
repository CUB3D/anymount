use std::io::Read;

use lzma_rust2::LzmaReader;
use memmap2::Mmap;

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};

pub struct LzmaF {
    pub f: Mmap,
    pub idx: usize,
}

impl GenFSProps for LzmaF {
    const FORMAT_NAME: &'static str = "lzma";
}

impl GenFS for LzmaF {
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
        if let Ok(mut reader) = LzmaReader::new_mem_limit(&f[..], u32::MAX, None) {
            let mut out = Vec::new();
            Ok(reader.read_to_end(&mut out).is_ok())
        } else {
            Ok(false)
        }
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        if self.idx == 1 {
            return Ok(None);
        }

        let mut reader = LzmaReader::new_mem_limit(&self.f[..], u32::MAX, None)?;
        let mut out = Vec::new();
        reader.read_to_end(&mut out)?;
        self.idx += 1;
        Ok(Some(Box::new(BufGenItm {
            name: "_dec".to_string(),
            data: out,
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
