//! OpenHarmony update.bin

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};
use memmap2::Mmap;
use parse::{le_u16, le_u32, le_u64, ne_u8, take};
use tracing::info;

pub struct OhosF {
    pub idx: usize,
    pub o: Vec<BufGenItm>,
}

impl GenFSProps for OhosF {
    const FORMAT_NAME: &'static str = "ohos";
}

impl GenFS for OhosF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        return Err(anyhow::anyhow!("ohos extract not supported yet"));

        let i = f.mmap;

        let mut comp = Vec::new();

        let mut k = &i[..];
        loop {
            let (mut j, tag) = le_u16(k)?;
            info!("{:x}", tag);
            match tag {
                // pkghdr
                0x01 => {
                    let (i, sz) = le_u16(j)?;
                    let (i, _) = take(i, sz as _)?;
                    j = i;
                }
                // time
                0x02 => {
                    let (i, sz) = le_u16(j)?;
                    let (i, _) = take(i, sz as _)?;
                    j = i;
                }
                // compinfo
                0x05 => {
                    let (i, sz) = le_u16(j)?;
                    let (i, cmp_info) = take(i, sz as _)?;
                    comp = cmp_info.to_vec();

                    let i = {
                        let mut x = i;
                        loop {
                            let (y, a) = ne_u8(x)?;
                            x = y;
                            if a == 0 {
                                break;
                            }
                        }
                        x
                    };

                    j = i;
                }
                // signinfo
                0x08 => {
                    // let (i, _) = le_u16(j)?;
                    let (i, sz) = le_u32(j)?;
                    let (i, _) = take(i, sz as _)?;
                    //todo: is this a versioning thing?
                    // let (i, _hash_check_data) = take(i, 115)?;
                    j = i;
                }
                // header
                0x11 => {
                    let (i, sz) = le_u16(j)?;
                    let (i, _) = take(i, sz as _)?;
                    j = i;
                    k = j;
                    break;
                }
                _ => panic!("{:x}", tag),
            }
            k = j;
        }
        let after_tlv = k;

        let mut comps = Vec::new();
        let mut i = &comp[..];
        while !i.is_empty() {
            println!("{}", i.len());
            #[derive(Debug)]
            struct CompInfo {
                id: u16,
                data_sz: u64,
                name: String,
            }
            let (j, name) = take(i, 32)?;

            // if name[0] != b'/'

            let (j, id) = le_u16(j)?;
            let (j, _res) = ne_u8(j)?;
            let (j, _flags) = ne_u8(j)?;
            let (j, _typ) = ne_u8(j)?;
            let (j, _version) = take(j, 10)?;
            let (j, data_sz) = le_u64(j)?;
            // let (j, data_sz) = le_u32(j)?;
            // let (j, _og_sz) = le_u32(j)?;
            let (j, _digest) = take(j, 32)?;

            let name_end = name.iter().position(|c| *c == b'\0').unwrap_or(name.len());
            let name = &name[..name_end];
            let name = String::from_utf8(name.to_vec())?;

            comps.push(CompInfo { name, id, data_sz });

            println!("{:?}", comps.last().unwrap());

            i = j;
        }

        let mut o = Vec::new();

        let mut i = after_tlv;
        for c in comps {
            if c.id != 48 {
                continue;
            }

            let (j, _data) = take(i, c.data_sz as _)?;
            i = j;

            info!("{} : {}", c.name, c.data_sz);

            o.push(BufGenItm {
                pos: 0,
                data: _data.to_vec(),
                name: format!("./{}.bin", c.name.clone()),
            });
        }

        Ok(Self { idx: 0, o })
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        let hdr = f.get(..2);

        if hdr != Some(0x01_u16.to_le_bytes().as_slice()) {
            return Ok(false);
        }

        Ok(true)
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        if let Some(i) = self.o.get(self.idx) {
            self.idx += 1;
            return Ok(Some(Box::new(i.clone())));
        }

        Ok(None)
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }
}

// For use in imhex
// struct compInfo {
//     char name[32];
//     u16 id;
//     u8 res;
//     u8 flags;
//     u8 typ;
//     char version[10];
//     u32 data_size;
//     u32 og_size;
//     u8 hash[32];
// };
//
// struct root {
//     u16 tag;
//     u16 len;
//     u32 c;
//     u32 d;
//     char magic[4];
//     u8 pad[60];
//     char name[64];
//     u16 e;
//     u16 f;
//     char date[16];
//     char time[16];
//     u16 g;
//     u16 cert_start;
// };
//
//
// compInfo a[59] @ 0xb4;
//
// struct tlv {
//     u16 tag;
//     if (tag == 8) {
//     u32 len;
//     } else {
//     u16 len;
//     }
//     if (tag == 5 && len > 5100) {
//     // data is actually fixed size here
//         u8 data[5149];
//     } else {
//      u8 data[len];
//     }
//     };
//
//     // types
//     // 1: update package
//     // 2: time
//     // 5: component
//
//     tlv pkg_hdr @ 0x0;
//
//     tlv version_info @ 0x8c;
//     tlv comp_info @ 0xb0;
//
//     tlv sign1 @ 0x14d1;
//     u8 hash_chk_data[115] @ 0x1dd4;
//    tlv idk1 @ 0x1e47;
//       u8 ptbl[a[3].data_size] @ 0x219d;
//       u8 next[a[4].data_size] @ 0x3319d;
//       u8 next2[a[5].data_size] @ 0x3319d + a[4].data_size;
//          u8 next3[a[6].data_size] @ 0x3319d + a[4].data_size + a[5].data_size;
//          u8 next4[a[7].data_size] @ 0x3319d + a[4].data_size + a[5].data_size + a[6].data_size;
