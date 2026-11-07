//! FTAB - Used in Apple C1 baseband firmware

use std::ops::Range;

use memmap2::Mmap;
use parse::{le_u32, take_arr, take_vec};
use crate::file_ref::FileRef;
use crate::gen_item::BufGenItm;
use crate::generic_fs_props::GenFSProps;
use crate::{gen_item::GenItem, generic_fs::GenFS};

struct Segment {
    name: String,
    data_range: Range<usize>,
}

pub struct FtabF {
    segments: Vec<Segment>,
    pos: usize,
    f: Mmap,
}

impl GenFSProps for FtabF {
    const FORMAT_NAME: &'static str = "ftab";
}

impl GenFS for FtabF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let (i, _unk0) = le_u32(&f.mmap)?;
        let (i, _unk1) = le_u32(i)?;
        let (i, _unk2) = le_u32(i)?;
        let (i, _unk3) = le_u32(i)?;
        let (i, ticket_offset) = le_u32(i)?;
        let (i, ticket_length) = le_u32(i)?;
        let (i, _unk4) = le_u32(i)?;
        let (i, _unk5) = le_u32(i)?;
        let (i, _magic) = take_arr::<8>(i)?;
        let (_, segment_count) = le_u32(i)?;
        let (i, _unk6) = le_u32(i)?;

        fn parse_seg(i: &[u8]) -> anyhow::Result<(&[u8], Segment)> {
            let (i, tag) = take_arr::<4>(i)?;
            let name = tag.map(|c| char::from_u32(c as u32).unwrap()).iter().collect::<String>();

            let (i, off) = le_u32(i)?;
            let (i, len) = le_u32(i)?;
            let (i, _unk) = le_u32(i)?;

            Ok((i, Segment {
                name,
                data_range: (off as usize)..(off as usize+len as usize),
            }))
        }

        let (_, mut segments) = take_vec(i, segment_count as usize, parse_seg)?;

        if !(ticket_offset == 0 && ticket_length == 0) {
            segments.push(Segment { name: "ticket".to_string(), data_range: (ticket_offset as usize..(ticket_offset + ticket_length) as usize) });
        }

        Ok(Self {
            segments,
            pos: 0,
            f: f.owned_map(),
        })
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        Ok(f.get(32..40) == Some(b"rkosftab".as_slice()))
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        let Some(entry) = self.segments.get(self.pos) else {
            return Ok(None);
        };
        self.pos += 1;

        Ok(Some(Box::new(BufGenItm::new(
            format!("{}.bin", entry.name),
            self.f[entry.data_range.clone()].to_vec(),
        ))))
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }
}