//! Unreal engine 3 UPX files
//! https://github.com/stricq/UPKManager/blob/master/UPK_Format.pdf

use crate::file_ref::FileRef;
use crate::gen_item::{BufGenItm, GenItem};
use crate::generic_fs::GenFS;
use crate::generic_fs_props::GenFSProps;
use memmap2::Mmap;
use parse::{le_i32, le_u32, le_u64, take, take_arr, take_vec};
use rust_lzo::{LZOContext, LZOError};

use aes::cipher::{BlockDecrypt, KeyInit};
use aes::Aes256;
use aes::cipher::generic_array::GenericArray;

pub struct UpxF {
    upx: UpxFile,
    idx: usize,
}

impl GenFSProps for UpxF {
    const FORMAT_NAME: &'static str = "upx";
}

fn fstring(i: &[u8]) -> anyhow::Result<(&[u8], String)> {
    let (i, len) = le_i32(i)?;
    assert!(len > 0); // ascii, null term
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
    pub namecnt: u32,
    pub nameoff: u32,
    pub expcnt: u32,
    pub expoff: u32,
    pub impcnt: u32,
    pub impoff: u32,
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
        let (i, namecnt) = le_u32(i)?;
        let (i, nameoff) = le_u32(i)?;
        let (i, expcnt) = le_u32(i)?;
        let (i, expoff) = le_u32(i)?;
        let (i, impcnt) = le_u32(i)?;
        let (i, impoff) = le_u32(i)?;
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
        if compflag & 0xf != 2 {
            anyhow::bail!("compressionFlags must be LZO {compflag:#x}");
        }

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
            namecnt,
            nameoff,
            expcnt,
            expoff,
            impcnt,
            impoff,
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
        };

        println!("{:x} {} {} {} {}", nameoff, expoff, impoff, depoff, seroff);

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

                let inp = &f.mmap[doff..][..comp_sz as usize];
                let (dec, err) =
                    LZOContext::decompress_to_slice(inp, &mut upx.content[cur..][..uncomp_sz as usize]);
                if err != LZOError::OK {
                    panic!("failed to decompress upx");
                }
                assert_eq!(dec.len(), uncomp_sz as usize);

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

        let foo = upx.chunks.iter().map(|c| c.uncom_of).min().unwrap() as usize;
        let mut full = f.mmap[..foo].to_vec();
        full.extend_from_slice(&upx.content[foo..]);

        let i = &upx.content[upx.expoff as usize..];
        let (i, ty) = le_u32(i)?;
        let (i, parent) = le_u32(i)?;
        let (i, owner) = le_u32(i)?;
        let (i, name) = le_u64(i)?;
        let (i, archtype) = le_u32(i)?;
        let (i, objflag1) = le_u32(i)?;
        let (i, objflag2) = le_u32(i)?;
        let (i, sz) = le_u32(i)?;
        let (_i, _off) = le_u32(i)?;

        println!("{ty} {parent} {owner} {name} {archtype} {objflag1} {objflag2} {sz}");


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
