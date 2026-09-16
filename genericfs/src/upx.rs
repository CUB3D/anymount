//! Unreal engine 3 UPX files
//! https://github.com/stricq/UPKManager/blob/master/UPK_Format.pdf
//! For bytecode: https://github.com/yole/unhood/blob/master/UnHood.Engine/BytecodeReader.cs#L193

use crate::file_ref::FileRef;
use crate::gen_item::{BufGenItm, GenItem};
use crate::generic_fs::GenFS;
use crate::generic_fs_props::GenFSProps;
use memmap2::Mmap;
use parse::{le_i32, le_u32, le_u64, ne_u8, take, take_arr, take_vec};
use rust_lzo::{LZOContext, LZOError};

use aes::cipher::{BlockDecrypt, KeyInit};
use aes::Aes256;
use aes::cipher::generic_array::GenericArray;
use tracing::warn;

pub struct UpxF {
    upx: UpxFile,
    idx: usize,
}

impl GenFSProps for UpxF {
    const FORMAT_NAME: &'static str = "upx";
}

fn fstring(i: &[u8]) -> anyhow::Result<(&[u8], String)> {
    let (i, len) = le_i32(i)?;
    assert!(len >= 0); // ascii, null term
    let (i, s) = take(i, len as usize)?;
    Ok((
        i,
        String::from_utf8_lossy(s)
            .trim_end_matches('\0')
            .to_string(),
    ))
}

#[derive(Debug)]
pub struct FCompChunk {
    pub uncom_of: u32,
    pub uncom_sz: u32,
    pub comp_sz: u32,
    pub comp_off: u32,
}

#[derive(Debug)]
pub struct Fgen {
    pub expcnt: u32,
    pub namecnt: u32,
    pub netobjcnt: u32,
}

#[derive(Debug)]
pub struct FNameEntry {
    pub name: String,
    pub flags: u64
}

fn read_fname_entry(i: &[u8]) -> anyhow::Result<(&[u8], FNameEntry)> {
    let (i, name) = fstring(i)?;
    let (i, flags) = le_u64(i)?;
    Ok((i, FNameEntry {
        name,
        flags,
    }))
}

#[derive(Debug)]
pub struct FObjectImport {
    pub pkg_idx: u64,
    pub type_idx: u64,
    pub owner_ref: u32,
    pub name_idx: u64
}

fn read_object_import(i: &[u8]) -> anyhow::Result<(&[u8], FObjectImport)> {
    let (i, pkg_idx) = le_u64(i)?;
    let (i, type_idx) = le_u64(i)?;
    let (i, owner_ref) = le_u32(i)?;
    let (i, name_idx) = le_u64(i)?;
    Ok((i, FObjectImport {
        pkg_idx,
        type_idx,
        owner_ref,
        name_idx,
    }))
}


#[derive(Debug)]
pub struct FObjectExport {
    pub type_ref: u32,
    pub parent_class_ref: u32,
    pub owner_ref: u32,
    pub name_idx: u64,
    pub archetype_ref: u32,
    pub objectflags_h: u32,
    pub objectflags_l: u32,
    pub serial_sz: u32,
    pub serial_off: u32,
    pub export_flags: u32,
    pub obj_count: u32,
    pub guid: [u8; 16],
    pub _unk: u32,
    pub data: Vec<u8>,
}

fn read_object_export(i: &[u8]) -> anyhow::Result<(&[u8], FObjectExport)> {
    let (i, type_ref) = le_u32(i)?;
    let (i, parent_class_ref) = le_u32(i)?;
    let (i, owner_ref) = le_u32(i)?;
    let (i, name_idx) = le_u64(i)?;
    let (i, archetype_ref) = le_u32(i)?;
    let (i, objectflags_h) = le_u32(i)?;
    let (i, objectflags_l) = le_u32(i)?;
    let (i, serial_sz) = le_u32(i)?;
    let (i, serial_off) = le_u32(i)?;
    let (i, export_flags) = le_u32(i)?;
    let (i, obj_count) = le_u32(i)?;
    let (i, guid) = take_arr::<16>(i)?;
    let (i, _unk) = le_u32(i)?;
    let (i, data) = take_vec(i, obj_count as usize * 4, ne_u8)?;

    Ok((i, FObjectExport {
        type_ref,
        parent_class_ref,
        owner_ref,
        name_idx,
        archetype_ref,
        objectflags_h,
        objectflags_l,
        serial_sz,
        serial_off,
        export_flags,
        obj_count,
        guid,
        _unk,
        data,
    }))
}

fn fgen(i: &[u8]) -> anyhow::Result<(&[u8], Fgen)> {
    let (i, expcnt) = le_u32(i)?;
    let (i, namecnt) = le_u32(i)?;
    let (i, netobjcnt) = le_u32(i)?;
    Ok((
        i,
        Fgen {
            expcnt,
            namecnt,
            netobjcnt,
        },
    ))
}

#[derive(Debug)]
pub struct UpxFile {
    pub ver: u32,
    pub hdrsz: u32,
    pub pkg: String,
    pub pkgflag: u32,
    pub name_count: u32,
    pub name_offset: u32,
    pub export_count: u32,
    pub export_offset: u32,
    pub import_count: u32,
    pub import_offset: u32,
    pub depoff: u32,
    pub seroff: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub guid: [u8; 16],
    pub gens: Vec<Fgen>,
    pub enginever: u32,
    pub cookver: u32,
    pub compflag: u32,
    pub chunks: Vec<FCompChunk>,
    pub content: Vec<u8>,

    pub name_table: Vec<FNameEntry>,
    pub import_table: Vec<FObjectImport>,
    pub export_table: Vec<FObjectExport>,
}

impl GenFS for UpxF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let (i, _sig) = le_u32(&f.mmap[..])?;
        let (i, ver) = le_u32(i)?;
        let (i, hdrsz) = le_u32(i)?;

        let (i, pkg) = fstring(i)?;

        let (i, pkgflag) = le_u32(i)?;
        let (i, name_count) = le_u32(i)?;
        let (i, name_offset) = le_u32(i)?;
        let (i, export_count) = le_u32(i)?;
        let (i, export_offset) = le_u32(i)?;
        let (i, import_count) = le_u32(i)?;
        let (i, import_offset) = le_u32(i)?;
        let (i, depoff) = le_u32(i)?;
        let (i, seroff) = le_u32(i)?;
        let (i, unk2) = le_u32(i)?;
        let (i, unk3) = le_u32(i)?;
        let (i, unk4) = le_u32(i)?;
        let (i, guid) = take_arr::<16>(i)?;

        let (i, num_gens) = le_u32(i)?;
        let (i, gens) = take_vec(i, num_gens as _, fgen)?;

        let (i, enginever) = le_u32(i)?;
        let (i, cookver) = le_u32(i)?;
        let (i, compflag) = le_u32(i)?;

        let (i, num_chunks) = le_u32(i)?;

        fn fcompchunk(i: &[u8]) -> anyhow::Result<(&[u8], FCompChunk)> {
            let (i, uncom_of) = le_u32(i)?;
            let (i, uncom_sz) = le_u32(i)?;
            let (i, comp_off) = le_u32(i)?;
            let (i, comp_sz) = le_u32(i)?;
            Ok((
                i,
                FCompChunk {
                    uncom_of,
                    uncom_sz,
                    comp_sz,
                    comp_off,
                },
            ))
        }

        let (_i, chunks) = take_vec(i, num_chunks as _, fcompchunk)?;

        let mut upx = UpxFile {
            ver,
            hdrsz,
            pkg,
            pkgflag,
            name_count,
            name_offset,
            export_count,
            export_offset,
            import_count,
            import_offset,
            depoff,
            seroff,
            unk2,
            unk3,
            unk4,
            guid,
            gens,
            enginever,
            cookver,
            compflag,
            chunks,
            content: Vec::new(),
            name_table: Vec::new(),
            import_table: Vec::new(),
            export_table: Vec::new(),
        };

        // We can't parse anything else here because we need to decrypt and decompress the chunks before any of the indexes have meaning

        for c in &upx.chunks {
            let base = c.comp_off as usize;
            if base + c.comp_sz as usize > f.mmap.len() {
                panic!("chunk too big")
            }

            let (i, tag) = le_u32(&f.mmap[base..])?;
            let (i, blk_sz) = le_u32(i)?;
            let (i, _total_comp) = le_u32(i)?;
            let (i, uncomp_sz) = le_u32(i)?;

            assert_eq!(tag, 0x9e2a83c1);
            assert_eq!(uncomp_sz, c.uncom_sz);

            let blk_cnt = (uncomp_sz + blk_sz - 1) / blk_sz;
            let end = c.uncom_of as usize + c.uncom_sz as usize;
            if upx.content.len() < end {
                upx.content.resize(end, 0);
            }

            let mut doff = base+16 + blk_cnt as usize * 8;
            let mut cur = c.uncom_of as usize;
            let mut i = i;
            for _bid in 0..blk_cnt {
                let (j, comp_sz) = le_u32(i)?;
                let (j, uncomp_sz) = le_u32(j)?;
                i = j;

                if upx.compflag & 0x2 != 0 {
                    let inp = &f.mmap[doff..][..comp_sz as usize];
                    let (dec, err) =
                        LZOContext::decompress_to_slice(inp, &mut upx.content[cur..][..uncomp_sz as usize]);
                    if err != LZOError::OK {
                        panic!("failed to decompress upx");
                    }
                    assert_eq!(dec.len(), uncomp_sz as usize);
                }

                doff += comp_sz as usize;
                cur += uncomp_sz as usize;
            }

            if upx.compflag & 0x200 != 0 {
                let start = c.uncom_of as usize;
                let enc_len = c.uncom_sz as usize & !0xf;

                let aes = Aes256::new_from_slice(b"sdjJKLJsklaJSLKJDWLZMXNsldjKjalk")?;
                for c in &mut upx.content[start..][..enc_len].chunks_exact_mut(16) {
                    aes.decrypt_block(GenericArray::from_mut_slice(c));
                }
            }
        }

        if upx.compflag == 0 {
            upx.content = f.mmap.to_vec();
        }


        // Read name table
        let (_, name_table) = take_vec(&upx.content[name_offset as usize..], name_count as usize, read_fname_entry)?;
        upx.name_table = name_table;

        let (_, import_table) = take_vec(&upx.content[import_offset as usize..], import_count as usize, read_object_import)?;
        upx.import_table = import_table;

        let (_, export_table) = take_vec(&upx.content[export_offset as usize..], export_count as usize, read_object_export)?;
        upx.export_table = export_table;

        for e in &upx.export_table {
            let n = match upx.name_table.get(e.name_idx as usize) {
                Some(n) => n,
                None => {
                    warn!("Failed to find name for {}", e.name_idx);
                    continue
                },
            };

            println!("Name: {}", n.name);

            let has_stack = e.objectflags_l & 0x02000000 != 0;

            let type_name = if (e.type_ref as i32) < 0 {
                let i = &upx.import_table[(-(e.type_ref as i32)) as usize];
                i.name_idx
            } else {
                let i = &upx.export_table[e.type_ref as usize];
                i.name_idx
            };
            let type_name = &upx.name_table[type_name as usize].name;
            println!("Type: {}", type_name);

            // if (e.parent_class_ref as i32) < 0 {
            //     let i = &upx.import_table[(-(e.parent_class_ref as i32)) as usize];
            //     println!("Parent Type: {}", &upx.name_table[i.name_idx as usize].name);
            // } else {
            //     let i = &upx.export_table[e.parent_class_ref as usize];
            //     println!("Parent Type: {}", &upx.name_table[i.name_idx as usize].name);
            // }

            let i = &upx.content[e.serial_off as usize..];

            if type_name == "Enum" {
                // UObject
                let (i, _net_idx) = le_u32(i)?;
                assert!(!has_stack);
                // UField
                let (i, _next_ref) = le_u32(i)?;

                // UEnum
                let (i, sz) = le_u32(i)?;
                println!("sz = {:x}", sz);
                let (_i, names) = take_vec(i, sz as usize, le_u64)?;
                for n in &names {
                    println!("Enum child: {}", &upx.name_table[*n as usize].name);

                }
            }

            if type_name == "Const" {
                // UObject
                let (i, _net_idx) = le_u32(i)?;
                assert!(!has_stack);
                // UField
                let (i, _next_ref) = le_u32(i)?;
                // UConst
                let (_, s) = fstring(i)?;
                println!("Value: {}", s);
            }

            if type_name == "ScriptStruct" {
                // UObject
                let (i, _net_idx) = le_u32(i)?;
                assert!(!has_stack);
                // UDefaultPorpertyList
                let (i, name_idx) = le_u32(i)?;
                println!("Def prop: {}", &upx.name_table[name_idx as usize].name);

                // UField
                let (i, _next_ref) = le_u32(i)?;
                // UStruct
                let (i, _script_text_ref) = le_u32(i)?;
                let (i, _first_child_ref) = le_u32(i)?;
                let (i, _cpp_text_ref) = le_u32(i)?;
                let (i, _line) = le_u32(i)?;
                let (i, _textpos) = le_u32(i)?;
                let (i, _scriptmemesz) = le_u32(i)?;
                let (i, scriptserialsz) = le_u32(i)?;
                let (_i, data) = take(i, scriptserialsz as usize)?;

                if !data.is_empty() && data.len() > 4 {
                    println!("Dat: {:x?}", data);
                }
            }

            if type_name == "Function" {
                // UObject
                let (i, _net_idx) = le_u32(i)?;
                assert!(!has_stack);
                // UDefaultPorpertyList
                let (i, name_idx) = le_u32(i)?;
                println!("Def prop: {}", &upx.name_table[name_idx as usize].name);
                // UField
                let (i, _next_ref) = le_u32(i)?;
                // UStruct
                let (i, _script_text_ref) = le_u32(i)?;
                let (i, _first_child_ref) = le_u32(i)?;
                let (i, _cpp_text_ref) = le_u32(i)?;
                let (i, _line) = le_u32(i)?;
                let (i, _textpos) = le_u32(i)?;
                let (i, _scriptmemesz) = le_u32(i)?;
                let (i, scriptserialsz) = le_u32(i)?;
                let (_i, data) = take(i, scriptserialsz as usize)?;


                if !data.is_empty() && data.len() > 4 {
                    println!("Dat: {:x?}", data);
                }
            }

            if type_name == "Class" {
                // UObject
                let (i, _net_idx) = le_u32(i)?;
                assert!(!has_stack);
                // UField
                let (i, _next_ref) = le_u32(i)?;
                // UStruct
                let (i, _script_text_ref) = le_u32(i)?;
                let (i, _first_child_ref) = le_u32(i)?;
                let (i, _cpp_text_ref) = le_u32(i)?;
                let (i, _line) = le_u32(i)?;
                let (i, _textpos) = le_u32(i)?;
                let (i, _scriptmemesz) = le_u32(i)?;
                let (i, scriptserialsz) = le_u32(i)?;
                let (_i, data) = take(i, scriptserialsz as usize)?;

                println!("{:x?}", data);

                if !data.is_empty() && data.len() > 4 {
                    panic!();
                }
            }

            let (_, x) = le_u32(i)?;
            println!("{:x}", x);

        }

        Ok(Self { upx, idx: 0 })
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        Ok(f.get(0..4) == Some(&[0xc1, 0x83, 0x2a, 0x9e]))
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        if self.idx == 1 {
            return Ok(None);
        }
        self.idx+=1;
        Ok(Some(Box::new(BufGenItm::new(format!("{}.u.dec", self.upx.pkg), self.upx.content.clone()))))
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }
}
