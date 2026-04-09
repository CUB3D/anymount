//! Huawei Update.app file
//! https://github.com/NDXCode/HW-Update-Extractor/blob/main/HW-Update-Extractor/MainWindow.xaml.cs

use parse::{le_u16, le_u32, take_arr};

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{gen_item::GenItem, generic_fs::GenFS};

pub struct UpdateAppF {}

impl GenFSProps for UpdateAppF {
    const FORMAT_NAME: &'static str = "update_app";
}

impl GenFS for UpdateAppF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let (i, _magic) = take_arr::<4>(f.mmap)?;
        let (i, _hdr_len) = le_u32(i)?;
        let (i, _idk) = le_u32(i)?;
        let (i, _id) = take_arr::<8>(i)?;
        let (i, _a) = le_u32(i)?;
        let (i, _sz) = le_u32(i)?;
        let (i, _date) = take_arr::<16>(i)?;
        let (i, _time) = take_arr::<8>(i)?;
        let (i, _typ) = take_arr::<8>(i)?;
        let (i, _chk) = le_u16(i)?;
        let (i, _bs) = le_u16(i)?;
        let (_i, _idk2) = le_u16(i)?;

        Ok(Self {})
    }
    fn sniff(f: &memmap2::Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        let (_i, magic) = take_arr::<4>(f)?;
        if magic != [0x55, 0xaa, 0x5a, 0xa5] {
            return Ok(false);
        }
        Ok(true)
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
