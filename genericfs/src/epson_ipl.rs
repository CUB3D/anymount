//! Epson IPL - Used in some printer firmwares

use crate::file_ref::FileRef;
use crate::gen_item::BufGenItm;
use crate::generic_fs_props::GenFSProps;
use crate::{gen_item::GenItem, generic_fs::GenFS};
use memmap2::Mmap;
use parse::be_u16;
use parse::le_u16;
use parse::le_u24;
use parse::ne_u8;

pub struct EpsonIPLF {
    pub idx: usize,
    pub out: Vec<BufGenItm>,
}

impl GenFSProps for EpsonIPLF {
    const FORMAT_NAME: &'static str = "epson_ipl";
}

impl GenFS for EpsonIPLF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let (i, tbl_sz) = be_u16(&f.mmap[0x0e..])?;


        let mut chunks = Vec::new();

        let mut j = i;
        for _ in 0..(tbl_sz / 0x10) {
            let (i, _unk1) = le_u16(j)?;
            let (i, _chksum) = le_u16(i)?;
            let (i, _unk2) = le_u16(i)?;
            let (i, _unk3) = le_u16(i)?;
            let (i, _unk4) = le_u16(i)?;
            let (i, _unk5) = le_u16(i)?;
            let (i, _unk6) = ne_u8(i)?;
            let (i, sz) = le_u24(i)?;

            if sz == 0 {
                break;
            }

            j = i;
            chunks.push(sz);
        }

        let mut out = Vec::new();
        let mut start = 0x1000;
        for (id, &sz) in chunks.iter().enumerate() {
            out.push(BufGenItm::new(format!("section_{id}"), f.mmap[start..][..sz as usize].to_vec()));
            start += sz as usize;
        }

        Ok(Self {out, idx: 0})
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        Ok(f.get(..10) == Some(b"\x0CEPSON IPL"))
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        if let Some(resource) = self.out.get(self.idx) {
            self.idx += 1;
            return Ok(Some(Box::new(resource.clone())));
        }

        Ok(None)
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }
}