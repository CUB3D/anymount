//! Apple IMG4 (iOS firmware)

use image4::{der::Decode, payload::compr_info::COMPR_ALGO_LZFSE, ImageRef};
use memmap2::Mmap;

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};

pub struct Img4F {
    pub f: Mmap,
    pub idx: usize,
}

impl GenFSProps for Img4F {
    const FORMAT_NAME: &'static str = "img4";
}

impl GenFS for Img4F {
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
        Ok(ImageRef::from_der(&f[..]).is_ok())
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        if self.idx == 1 {
            return Ok(None);
        }

        let image = ImageRef::from_der(&self.f[..])?;
        let payload = image.payload();
        let data = payload.data();

        let mut d = Vec::new();
        if let Some(info) = payload.compr_info() {
            match info.algo {
                COMPR_ALGO_LZFSE => {
                    lzfse_rust::decode_bytes(data, &mut d)?;
                }
                _ => {
                    return Err(anyhow::anyhow!("invalid img4 compression type"));
                }
            }
        } else {
            d.extend_from_slice(data);
        }

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