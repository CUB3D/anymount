//! HMFS

use memmap2::Mmap;
use parse::{le_u32, take_arr, take_cstr_utf8, take_vec};

use crate::file_ref::FileRef;
use crate::gen_item::BufGenItm;
use crate::generic_fs_props::GenFSProps;
use crate::{gen_item::GenItem, generic_fs::GenFS};

struct DirEnt {
    name: String,
    len: u32,
    data: Vec<u8>,
}

fn read_dir(i: &[u8]) -> anyhow::Result<(&[u8], DirEnt)> {
    let (i, name) = take_arr::<107>(i)?;
    let (i, _padding) = take_arr::<5>(i)?;
    let (i, len) = le_u32(i)?;
    let (i, _unk1) = take_arr::<12>(i)?;

    let (_i, name) = take_cstr_utf8(&name)?;

    Ok((
        i,
        DirEnt {
            name,
            len,
            data: Vec::new(),
        },
    ))
}

pub struct HmfsF {
    idx: usize,
    entries: Vec<DirEnt>,
}

impl GenFSProps for HmfsF {
    const FORMAT_NAME: &'static str = "hmfs";
}

impl GenFS for HmfsF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let (i, _magic) = take_arr::<4>(f.mmap)?;
        let (i, _unk1) = le_u32(i)?;
        let (_i, cnt) = le_u32(i)?;

        let file_table = &f.mmap[0x80..];
        let (_i, mut entries) = take_vec(file_table, cnt as usize, read_dir)?;

        let mut data_offset = 0x2000;
        for ent in &mut entries {
            ent.data
                .extend_from_slice(&f.mmap[data_offset..][..ent.len as usize]);

            let rounded_length = ent.len + 0x1000;
            let rounded_length = rounded_length - ent.len % 0x1000;

            data_offset += rounded_length as usize;
        }

        Ok(Self { idx: 0, entries })
    }
    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        if f.get(..4) == Some(b"HMFS") {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        if let Some(i) = self.entries.get(self.idx) {
            self.idx += 1;
            return Ok(Some(Box::new(BufGenItm::new(&i.name, i.data.clone()))));
        }

        Ok(None)
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }
}
