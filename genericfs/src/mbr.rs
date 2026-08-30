//! Master Boot Record

use mbrman::MBR;
use memmap2::Mmap;
use std::io::Cursor;

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};

pub struct MbrF {
    idx: usize,
    mbr: MBR,
    mmap: Mmap,
}

impl GenFSProps for MbrF {
    const FORMAT_NAME: &'static str = "mbr";
}

impl GenFS for MbrF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let mut m = f.owned_file();
        let mbr = MBR::read_from(&mut m, 512)?;

        Ok(Self {
            idx: 0,
            mbr,
            mmap: f.owned_map(),
        })
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        let mut c = Cursor::new(&f[..]);
        if MBR::read_from(&mut c, 512).is_ok() {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        if self.idx == 0 {
            self.idx += 1;
            return Ok(Some(Box::new(BufGenItm::new(
                "_mbr_info.txt",
                format!("{:?}", self.mbr).as_bytes().to_vec(),
            ))));
        } else if let Some((pidx, p)) = self.mbr.iter().nth(self.idx - 1) {
            if p.sectors > 0 {
                let size = p.sectors as u64 * self.mbr.sector_size as u64;
                // println!("sectors: {}", p.sectors);
                // println!("sectors: {}", mbr.sector_size);
                // println!("size: {}", size);

                let past_header = &self.mmap[512..];

                let mut d = vec![0u8; size as usize];
                let file_size = size.min(past_header.len() as u64);
                let data = &past_header[p.starting_lba as usize * self.mbr.sector_size as usize..]
                    [..file_size as usize];
                d[..file_size as usize].copy_from_slice(data);

                return Ok(Some(Box::new(BufGenItm::new(
                    format!("partition_{}", pidx),
                    d,
                ))));
            }

            self.idx += 1;
        }

        Ok(None)
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }
}
