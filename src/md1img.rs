//! mtk md1rom file
//! https://github.com/R0rt1z2/md1imgpy/blob/main/md1imgpy/structures/header.py

use memmap2::Mmap;
use parse::{le_u32, take, take_all, take_arr, take_cstr_utf8};

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};

#[derive(Debug)]
struct Md1Ent {
    name: String,
    data: Vec<u8>,
}

pub struct Md1imgF {
    idx: usize,
    files: Vec<Md1Ent>,
}

impl GenFSProps for Md1imgF {
    const FORMAT_NAME: &'static str = "mtk_md1img";
}

impl GenFS for Md1imgF {
    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        if f.get(..4) == Some(&[0x88u8, 0x16, 0x88, 0x58]) {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        pub fn take_md1ent(i: &[u8]) -> anyhow::Result<(&[u8], Md1Ent)> {
            let start = i;
            let (i, magic) = take_arr::<4>(i)?;
            println!("{:x?}", magic);

            if magic != [0x88, 0x16, 0x88, 0x58] {
                anyhow::bail!("Bad magic on section header");
            }

            let (i, dsz) = le_u32(i)?;
            let (i, name) = take_arr::<32>(i)?;
            let (i, _base) = le_u32(i)?;
            let (i, _mode) = le_u32(i)?;
            let (i, _magic2) = le_u32(i)?;
            let (i, doff) = le_u32(i)?;
            let (i, _ver) = le_u32(i)?;
            let (i, _typ) = le_u32(i)?;
            let (i, _img_end) = le_u32(i)?;
            let (i, _align) = le_u32(i)?;
            let (i, _dsz_ext) = le_u32(i)?;
            let (i, _maddr_ext) = le_u32(i)?;
            let (_i, _res) = take_arr::<432>(i)?;

            let i = &start[doff as usize..];
            let (i, dat) = take(i, dsz as usize)?;

            let mut align = (doff + dsz) % 16;
            if align == 0 {
                align = 0;
            } else {
                align = 16 - align;
            }
            let i = &i[align as usize..];

            let (_, name) = take_cstr_utf8(&name)?;

            Ok((
                i,
                Md1Ent {
                    name,
                    data: dat.to_vec(),
                },
            ))
        }

        let files = take_all(f.mmap, take_md1ent)?;

        Ok(Self { idx: 0, files })
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        if let Some(i) = self.files.get(self.idx) {
            self.idx += 1;
            return Ok(Some(Box::new(BufGenItm::new(&i.name, i.data.clone()))));
        }

        Ok(None)
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }
}
