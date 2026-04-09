//! Unreal engine 3 UPX files
//! https://github.com/stricq/UPKManager/blob/master/UPK_Format.pdf

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{gen_item::GenItem, generic_fs::GenFS};
use memmap2::Mmap;
use parse::{le_i32, le_u32, take, take_arr, take_vec};
use rust_lzo::LZOContext;

pub struct UpxF;

impl GenFSProps for UpxF {
    const FORMAT_NAME: &'static str = "upx";
}

fn fstring(i: &[u8]) -> anyhow::Result<(&[u8], String)> {
    let (i, len) = le_i32(i)?;
    assert!(len > 0); // ascii, null term
    let (i, s) = take(i, len as usize)?;
    Ok((i, String::from_utf8_lossy(s).to_string()))
}

#[derive(Debug)]
pub struct FCompChunk {
    pub uncom_of: u32,
    pub uncom_sz: u32,
    pub comp_sz: u32,
    pub comp_off: u32,
}

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
}

impl GenFS for UpxF {
    fn try_open_internal(_f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {})
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        let i = &f[..];

        let (i, sig) = le_u32(i)?;
        if sig != 0x9e2a83c1 {
            return Ok(false);
        }

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
        assert_eq!(compflag, 0x202); // Dirty Bomb, LZO1X

        let (i, num_chunks) = le_u32(i)?;
        let (_i, chunks) = take_vec(i, num_chunks as _, fcompchunk)?;
        println!("{:?}", chunks);
        assert_eq!(chunks.len(), 1);

        let str = UpxFile {
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
        };

        // for c in chunks {
        //     let mut out = vec![0u8; c.uncom_sz as _];
        //     let inp = &f[c.comp_off as usize-1..][..c.comp_sz as usize];
        //
        //     let mut tmp = vec![0u8; 0x20000];
        //     tmp[..inp.len()].copy_from_slice(inp);
        //
        //     let dec = lzo1x::decompress(&tmp, &mut out);
        //     println!("dec res - {:?}", dec);

        let _fchunk = str.chunks.first().unwrap();

        let mut out = vec![0u8; 226078];
        for off in 0..f.len() {
            let mut v = vec![0u8; 226078];
            out.fill(0);

            let inp = &f[off..];
            let inp = if inp.len() > v.len() {
                &inp[..v.len()]
            } else {
                inp
            };
            v[..inp.len()].copy_from_slice(inp);

            // let x = lzo1x::decompress(&v, &mut out);
            let (dec, err) = LZOContext::decompress_to_slice(&v, &mut out);

            if off < 500 {
                std::fs::write("./test.bin", &dec).unwrap();
            }

            let err = err as i32;
            if err == 0 || err == -8 {
                println!("{:?}", off);
            }
        }
        // }

        //TODO: we can read the hdr fine
        // we can't do the decomp, some libs are too strict and this one doesn't produce enough output
        // It might be worth hooking the lzo func with frida and dumping what the actual args are

        Ok(true)
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        Ok(None)
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }
}
