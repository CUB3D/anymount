//! LZO

use memmap2::Mmap;

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};

pub struct LzoF {
    pub f: Mmap,
    pub idx: usize,
}

impl GenFSProps for LzoF {
    const FORMAT_NAME: &'static str = "lzo";
}

impl GenFS for LzoF {
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
        Ok(f.get(..9) == Some(b"\x89LZO\x00\r\n\x1a\n"))
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        if self.idx == 1 {
            return Ok(None);
        }

        let mut out = vec![0u8; 0x1000];
        loop {
            match lzokay::decompress::decompress(&self.f, &mut out) {
                Ok(n) => {
                    out.truncate(n);
                    break;
                }
                Err(lzokay::Error::OutputOverrun) => {
                    out.reserve(out.len());
                    out.clear();
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Decompress failed: {e}"))
                },
            }
        }

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