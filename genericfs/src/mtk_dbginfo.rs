//! mtk md1rom file
//! https://github.com/R0rt1z2/md1imgpy/blob/main/md1imgpy/structures/header.py

use memmap2::Mmap;
use parse::{Take, le_u16, le_u32, take_arr, take_cstr_utf8, take_until, take_vec};
use serde::Serialize;

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};

#[derive(Debug, Serialize)]
struct FunctionSymbol {
    name: String,
    start_addr: u32,
    end_addr: u32,
}

#[derive(Debug, Serialize)]
struct FileSymbol {
    name: String,
    data: Vec<[u16; 4]>,
}

#[derive(Serialize)]
struct MtkDebugFile {
    functions: Vec<FunctionSymbol>,
    files: Vec<FileSymbol>,
    timestamp: String,
    modem_ver: String,
    tgt: String,
    platform: String,
}

pub struct MtkDbgF {
    idx: usize,
    data: MtkDebugFile,
}

impl GenFSProps for MtkDbgF {
    const FORMAT_NAME: &'static str = "mtk_dbg";
}

impl GenFS for MtkDbgF {
    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        if f.get(..4) == Some(&[0x43u8, 0x41, 0x54, 0x49]) {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let (i, _idk) = take_arr::<0x1c>(f.mmap)?;
        let (i, tgt) = take_cstr_utf8(i)?;
        let (i, platform) = take_cstr_utf8(i)?;
        let (i, modem_ver) = take_cstr_utf8(i)?;
        let (i, timestamp) = take_cstr_utf8(i)?;

        let (i, functions_offset) = le_u32(i)?;
        let (_i, files_offset) = le_u32(i)?;

        let functions = &f.mmap[functions_offset as usize..];
        let (i, _idk2) = take_arr::<0x10>(functions)?;

        fn sym(i: &[u8]) -> anyhow::Result<(&[u8], Take<FunctionSymbol>)> {
            let (i, name) = take_cstr_utf8(i)?;
            if name.is_empty() {
                return Ok((i, Take::End));
            }
            let (i, start) = le_u32(i)?;
            let (i, end) = le_u32(i)?;

            Ok((
                i,
                Take::More(FunctionSymbol {
                    name,
                    start_addr: start,
                    end_addr: end,
                }),
            ))
        }
        let (_i, functions) = take_until(i, sym)?;

        let file_smybols = &f.mmap[files_offset as usize..];
        let (i, _idk3) = take_arr::<0x10>(file_smybols)?;

        fn file_sym(i: &[u8]) -> anyhow::Result<(&[u8], Take<FileSymbol>)> {
            let (i, name) = take_cstr_utf8(i)?;
            if name.is_empty() {
                return Ok((i, Take::End));
            }

            let (i, cnt) = le_u32(i)?;

            fn file_sym_data(i: &[u8]) -> anyhow::Result<(&[u8], [u16; 4])> {
                let (i, a0) = le_u16(i)?;
                let (i, a1) = le_u16(i)?;
                let (i, a2) = le_u16(i)?;
                let (i, a3) = le_u16(i)?;

                Ok((i, [a0, a1, a2, a3]))
            }

            let (i, data) = take_vec(i, cnt as usize, file_sym_data)?;

            Ok((i, Take::More(FileSymbol { name, data })))
        }
        let (_i, files) = take_until(i, file_sym)?;

        Ok(Self {
            idx: 0,
            data: MtkDebugFile {
                files,
                functions,
                tgt,
                timestamp,
                modem_ver,
                platform,
            },
        })
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        if self.idx == 0 {
            self.idx = 1;
            let data = serde_json::to_string_pretty(&self.data)?;
            return Ok(Some(Box::new(BufGenItm::new(
                "mtk_debug_info.json",
                data.as_bytes().to_vec(),
            ))));
        }

        Ok(None)
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }
}
