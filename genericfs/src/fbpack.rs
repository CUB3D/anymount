//! fbpack ("FastBoot PacK") - used by Pixel modems
//! https://github.com/LineageOS/scripts/tree/main/fbpacktool - Qualcomm modem pixel 5 mar 2023

use memmap2::Mmap;
use parse::{le_u32, le_u64, take_arr, take_cstr_utf8, take_vec};

use crate::file_ref::FileRef;
use crate::gen_item::BufGenItm;
use crate::generic_fs_props::GenFSProps;
use crate::{gen_item::GenItem, generic_fs::GenFS};

#[derive(Debug)]
#[expect(dead_code)]
struct PackEntV2 {
    ty: u32,
    name: String,
    prod: [u8; 40],
    off: u64,
    sz: u64,
    slot: u32,
    crc: u32,
}

#[derive(Debug)]
#[expect(dead_code)]
struct PackEntV1 {
    ty: u32,
    name: String,
    sz_h: u32,
    sz: u32,
    next_off_h: u32,
    next_off: u32,
    crc: u32,

    data_offset: u64,
}

pub struct FbpackF {
    pos: usize,
    version: u32,
    entries_v1: Vec<PackEntV1>,
    entries: Vec<PackEntV2>,
    f: Mmap,
}

impl GenFSProps for FbpackF {
    const FORMAT_NAME: &'static str = "fbpack";
}

impl GenFS for FbpackF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let (i, _magic) = le_u32(f.mmap)?;
        let (i, version) = le_u32(i)?;
        if version == 1 {
            let (i, _img_ver) = take_arr::<68>(i)?;
            let (i, ent_cnt) = le_u32(i)?;
            let (i, _sz) = le_u32(i)?;

            //TODO: this might be buggy, test on pixel11 radio.img

            fn read_pack_ent_v1(i: &[u8]) -> anyhow::Result<(&[u8], PackEntV1)> {
                let (i, ty) = le_u32(i)?;
                let (i, name) = take_arr::<32>(i)?;
                let (i, sz_h) = le_u32(i)?;
                let (i, sz) = le_u32(i)?;
                let (i, next_off_h) = le_u32(i)?;
                let (i, next_off) = le_u32(i)?;
                let (i, crc) = le_u32(i)?;

                let (_, name) = take_cstr_utf8(&name)?;

                Ok((
                    i,
                    PackEntV1 {
                        ty,
                        name,
                        sz_h,
                        sz,
                        next_off_h,
                        next_off,
                        crc,
                        data_offset: 0,
                    },
                ))
            }

            let mut i = i;
            let mut entries = Vec::new();
            let mut data_offset = 4 + 4 + 68 + 4 + 4;
            for ii in 0..ent_cnt {
                let (_, mut ent) = read_pack_ent_v1(i)?;
                println!("{:?}", ent);
                data_offset += 4 + 32 + 4 + 4 + 4 + 4 + 4;
                ent.data_offset = data_offset;

                let next_off = ((ent.next_off_h as u64) << 32) | (ent.next_off as u64);
                // Last one is oob
                if ii < ent_cnt - 1 {
                    i = &f.mmap[next_off as usize..];
                }
                entries.push(ent);
                data_offset = next_off;
            }

            Ok(Self {
                version,
                pos: 0,
                entries: Vec::new(),
                entries_v1: entries,
                f: f.owned_map(),
            })
        } else if version == 2 {
            let (i, _header_sz) = le_u32(i)?;
            let (i, _entry_header_sz) = le_u32(i)?;
            let (i, _pltform) = take_arr::<16>(i)?;
            let (i, _ver) = take_arr::<64>(i)?;
            let (i, _typ) = le_u32(i)?;
            let (i, _align) = le_u32(i)?;
            let (i, ent_cnt) = le_u32(i)?;
            let (i, _sz) = le_u32(i)?;

            fn read_pack_ent_v2(i: &[u8]) -> anyhow::Result<(&[u8], PackEntV2)> {
                let (i, ty) = le_u32(i)?;
                let (i, name) = take_arr::<36>(i)?;
                let (i, prod) = take_arr::<40>(i)?;
                let (i, off) = le_u64(i)?;
                let (i, sz) = le_u64(i)?;
                let (i, slot) = le_u32(i)?;
                let (i, crc) = le_u32(i)?;

                let (_, name) = take_cstr_utf8(&name)?;

                Ok((
                    i,
                    PackEntV2 {
                        ty,
                        name,
                        prod,
                        off,
                        sz,
                        slot,
                        crc,
                    },
                ))
            }

            let (_, entries) = take_vec(i, ent_cnt as usize, read_pack_ent_v2)?;

            Ok(Self {
                version,
                pos: 0,
                entries,
                entries_v1: Vec::new(),
                f: f.owned_map(),
            })
        } else {
            todo!();
        }
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        let (i, magic) = le_u32(f)?;
        let (_i, version) = le_u32(i)?;

        // FBPK
        //TODO: bootldr format
        if magic == 0x4b504246 && (version == 1 || version == 2) {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        if self.version == 2 {
            if let Some(e) = self.entries.get(self.pos) {
                self.pos += 1;

                match e.ty {
                    // Partition table entry
                    0 => Ok(None),
                    _ => {
                        let data = self.f[e.off as usize..][..e.sz as usize].to_vec();
                        Ok(Some(Box::new(BufGenItm::new(e.name.clone(), data))))
                    }
                }
            } else {
                Ok(None)
            }
        } else if self.version == 1 {
            if let Some(e) = self.entries_v1.get(self.pos) {
                self.pos += 1;

                match e.ty {
                    // Partition table entry
                    0 => Ok(None),
                    _ => {
                        let sz = ((e.sz_h as u64) << 32) | (e.sz as u64);
                        let mut data = vec![0u8; sz as usize];

                        // Pad end with 0
                        let fdata = &self.f[e.data_offset as usize..];
                        let fsz = sz.min(fdata.len() as u64);
                        let fdata = &fdata[..fsz as usize];
                        data[..fsz as usize].copy_from_slice(fdata);
                        Ok(Some(Box::new(BufGenItm::new(e.name.clone(), data))))
                    }
                }
            } else {
                Ok(None)
            }
        } else {
            todo!()
        }
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }
}
