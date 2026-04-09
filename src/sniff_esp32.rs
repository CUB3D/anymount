//! ESP32 fw file, sniff only
//! https://docs.espressif.com/projects/esptool/en/latest/esp32/advanced-topics/firmware-image-format.html

use memmap2::Mmap;
use parse::{le_u16, le_u32, ne_u8, take, take_arr};

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{gen_item::GenItem, generic_fs::GenFS};

struct EspHeader {
    hash_appended: bool,
}

fn parse_hdr(i: &[u8]) -> anyhow::Result<(&[u8], EspHeader)> {
    let (i, magic) = ne_u8(i)?;
    if magic != 0xe9 {
        anyhow::bail!("Incorrect magic");
    }
    let (i, seg_cnt) = ne_u8(i)?;
    let (i, _flash_mode) = ne_u8(i)?;
    let (i, _size_freq) = ne_u8(i)?;
    let (i, _entry_addr) = le_u32(i)?;
    // ext hdr
    let (i, _wp) = ne_u8(i)?;
    let (i, _settings) = take_arr::<3>(i)?;
    let (i, _chip_id) = le_u16(i)?;
    let (i, _min_rev_deprecated) = ne_u8(i)?;
    let (i, _min_rev_1) = le_u16(i)?;
    let (i, _maj_rev) = le_u16(i)?;
    let (i, _res) = take_arr::<4>(i)?;
    let (i, hash_appended) = ne_u8(i)?;

    let mut j = i;
    for _ in 0..seg_cnt {
        let (i, _off) = le_u32(j)?;
        let (i, sz) = le_u32(i)?;
        let (i, _data) = take(i, sz as usize)?;
        j = i;
    }
    Ok((
        i,
        EspHeader {
            hash_appended: hash_appended == 1,
        },
    ))
}

pub struct Esp32F {}

impl GenFSProps for Esp32F {
    const FORMAT_NAME: &'static str = "esp32_fw";
}

impl GenFS for Esp32F {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let start_sz = f.mmap.len();
        //hdr
        let (i, hdr) = parse_hdr(f.mmap)?;

        let amt_read = start_sz - i.len();
        let adjust = amt_read % 16;
        let adjust = 15 - adjust;
        let (i, _pad) = take(i, adjust)?;
        let (i, _chk) = ne_u8(i)?;

        if hdr.hash_appended {
            let (_i, _hash) = take_arr::<32>(i)?;
        }

        Ok(Self {})
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        Ok(parse_hdr(&f[..]).is_ok())
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        Ok(None)
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }

    fn sniff_only(&self) -> bool {
        true
    }
}
