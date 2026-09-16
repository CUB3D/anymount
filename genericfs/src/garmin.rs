//! Garmin IMG files

use parse::{le_u16, le_u32, ne_u8, take, take_until, Take};

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{gen_item::{BufGenItm, GenItem}, generic_fs::GenFS};
use memmap2::Mmap;

struct FatEntry {
    name: String,
    typ: String,
    size: usize,
    blocks: Vec<u16>,
}

pub struct GarminF {
    mmap: Mmap,
    block_size: usize,
    entries: Vec<FatEntry>,
    idx: usize,
}

impl GenFSProps for GarminF {
    const FORMAT_NAME: &'static str = "garmin";
}

impl GenFS for GarminF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let hdr = &f.mmap[..];
        let (i, _idk) = ne_u8(hdr)?;
        let (i, _) = take(i, 7)?;

        let (i, _ver_major) = ne_u8(i)?;
        let (i, _ver_minor) = ne_u8(i)?;
        let (i, _upd_month) = ne_u8(i)?;
        let (i, _upd_year) = ne_u8(i)?;

        let (i, _idk2) = take(i, 2)?;
        let (i, _mapsource) = ne_u8(i)?;
        let (i, _chk) = ne_u8(i)?;
        let (i, _sig) = take(i, 7)?;

        let (i, _) = take(i, 41)?;
        let (i, phys_blk) = ne_u8(i)?;

        let (i, _name) = take(i, 7)?;
        let (i, _idk3) = ne_u8(i)?;
        let (i, _map_desc) = take(i, 20)?;
        let (i, _head) = le_u16(i)?;
        let (i, _sec) = le_u16(i)?;
        let (i, blocksize_1) = ne_u8(i)?;
        let (_, blocksize_2) = ne_u8(i)?;

        let blk_sz = 1 << (blocksize_1 + blocksize_2);
        let start_addr = (phys_blk as usize + 1) * 0x200;

        let mut entries  = Vec::<FatEntry>::new();
        let mut fat_off = start_addr;

        fn read_fat_ent(i: &[u8]) -> anyhow::Result<Option<(String, String, usize, Vec<u16>, bool)>> {
            let (i, eof_marker) = ne_u8(i)?;
            if eof_marker == 0 {
                return Ok(None);
            }

            let (i, name) = take(i, 8)?;
            let tmp = name.iter().position(|&c| c == 0).unwrap_or(name.len());
            let name = String::from_utf8_lossy(&name[..tmp]).to_string();

            let (i, typ) = take(i, 3)?;
            let tmp = typ.iter().position(|&c| c == 0).unwrap_or(typ.len());
            let typ = String::from_utf8_lossy(&typ[..tmp]).to_string();

            let (i, size) = le_u32(i)?;
            let (i, next_fat) = le_u16(i)?;
            let (i, _idk1) = take(i, 14)?;

            let (_, blocks) = take_until(&i[..240*2], |i| -> anyhow::Result<(&[u8], Take<u16>)> {
                let (i, x) = le_u16(i)?;
                if x == 0xffff {
                    Ok((i, Take::End))
                } else {
                    if i.is_empty() {
                        Ok((i, Take::Last(x)))
                    } else {
                        Ok((i, Take::More(x)))
                    }
                }
            })?;

            Ok(Some((name, typ, size as usize, blocks, next_fat != 0)))
        }

        while let Some((name, typ, size, blocks, continuation)) = read_fat_ent(&f.mmap[fat_off..])? {

            if continuation {
                entries.last_mut().expect("No prev subfile").blocks.extend(blocks);
            } else {
                entries.push(FatEntry {
                    name,
                    typ,
                    size,
                    blocks,
                });
            }

            fat_off += 0x200;
        }

        Ok(Self {
            mmap: f.owned_map(),
            block_size: blk_sz,
            entries,
            idx: 0,
        })
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        Ok(f.get(0x10..0x17) == Some(b"DSKIMG\0"))
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        if let Some(e) = self.entries.get(self.idx) {
            self.idx += 1;

            let mut data = Vec::new();
            for &blk in &e.blocks {
                let seg = &self.mmap[blk as usize * self.block_size..][..self.block_size];
                data.extend_from_slice(&seg[..seg.len().min(e.size - data.len())]);
                if data.len() >= e.size {
                    break;
                }
            }
            data.truncate(e.size);

            Ok(Some(Box::new(BufGenItm::new(format!("{}.{}", e.name, e.typ), data))))
        } else {
            Ok(None)
        }
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }
}