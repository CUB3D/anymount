//! Linux zimage

use flate2::read::GzDecoder;
use memmap2::Mmap;
use std::io::Read;

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};

pub struct LinuzZImgF {
    pub idx: usize,
    pub o: Vec<BufGenItm>,
}

impl GenFSProps for LinuzZImgF {
    const FORMAT_NAME: &'static str = "linux_zimg";
}

impl GenFS for LinuzZImgF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let v = &f.mmap[..];

        let tgt = [0x1Fu8, 0x8B, 0x08, 0x00];
        let tgt2 = [0x1Fu8, 0x8B, 0x08, 0x08];

        let mut pos = None;
        for i in 0..v.len() - 4 {
            if v[i..][..4] == tgt || v[i..][..4] == tgt2 {
                pos = Some(i);
                break;
            }
        }
        if pos.is_none() {
            return Err(anyhow::anyhow!("LinuzZImgF index out of range"));
        }

        let pos = pos.unwrap();
        let mut g = GzDecoder::new(&v[pos..]);
        let mut dec = Vec::new();
        g.read_to_end(&mut dec)?;

        let mut o = Vec::new();
        o.push(BufGenItm::new("_kernel_decompressed", dec.clone()));

        //TODO: this actually works on any kernel not just zimg
        let ikcfg_st = [0x49u8, 0x4B, 0x43, 0x46, 0x47, 0x5F, 0x53, 0x54];
        let ikcfg_end = [0x49u8, 0x4B, 0x43, 0x46, 0x47, 0x5F, 0x45, 0x44];

        let v = dec;

        let mut st_pos = None;
        for i in 0..v.len() - ikcfg_st.len() {
            if v[i..][..ikcfg_st.len()] == ikcfg_st {
                st_pos = Some(i);
                break;
            }
        }

        let mut ed_pos = None;
        for i in 0..v.len() - ikcfg_end.len() {
            if v[i..][..ikcfg_end.len()] == ikcfg_end {
                ed_pos = Some(i);
                break;
            }
        }

        if let (Some(st), Some(ed)) = (st_pos, ed_pos) {
            let cfg_compressed = v[st + ikcfg_st.len()..ed].to_vec();

            let mut dec = Vec::new();
            GzDecoder::new(&cfg_compressed[..]).read_to_end(&mut dec)?;

            o.push(BufGenItm::new("_config", dec));
        }

        Ok(Self { o, idx: 0 })
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        let hdr = f
            .get(..16)
            .ok_or_else(|| anyhow::anyhow!("Not enough data"))?;

        if hdr
            != [
                0x00, 0x00, 0xA0, 0xE1, 0x00, 0x00, 0xA0, 0xE1, 0x00, 0x00, 0xA0, 0xE1, 0x00, 0x00,
                0xA0, 0xE1,
            ]
        {
            return Ok(false);
        }

        Ok(true)
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        if let Some(i) = self.o.get(self.idx) {
            self.idx += 1;
            return Ok(Some(Box::new(i.clone())));
        }

        Ok(None)
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }
}
