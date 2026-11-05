//! Epson RCX - Used in some printer firmwares

use std::collections::BTreeMap;
use memmap2::Mmap;
use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{gen_item::GenItem, generic_fs::GenFS};
use crate::gen_item::BufGenItm;

pub struct EpsonRcxF {
    pub idx: usize,
    pub out: Vec<BufGenItm>,
}

impl GenFSProps for EpsonRcxF {
    const FORMAT_NAME: &'static str = "epson_rcx";
}

impl GenFS for EpsonRcxF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let end = f.mmap.iter().position(|o| *o == 0xc).unwrap();
        let hdr = &f.mmap[..end as usize];
        println!("{hdr:?}");
        let hdr = String::from_utf8(hdr.to_vec())?;
        println!("{hdr:?}");

        let mut data = BTreeMap::<String, u32>::default();
        let mut cur_section = None;

        for l in hdr.lines() {
            if l.starts_with("[") {
                cur_section = Some(l[1..].split(']').next().unwrap().to_string());
            } else if l.contains("=") && let Some(cur_section) = cur_section.clone() {
                let parts = l.split("=").collect::<Vec<_>>();

                if ["A", "B", "D"].contains(&cur_section.as_str()) {
                    continue;
                }

                let tag = parts[0].to_string();
                let val = parts[1].replace("\"", "");
                if tag == "5" {
                    println!("{cur_section}:{tag} = {val:?}");
                    let val = val.parse::<u32>()?;
                    *data.entry(cur_section).or_default() = val;
                }
            }
        }

        let mut out = Vec::new();
        let mut start = end;
        for (section, size) in data {
            let dat = f.mmap[start..][..size as usize].to_vec();
            out.push(BufGenItm::new(section, dat));
            start += size as usize;
        }

        Ok(Self {out, idx: 0})
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        Ok(f.get(..5) == Some(b"RCX\r\n"))
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        if let Some(resource) = self.out.get(self.idx) {
            self.idx += 1;
            return Ok(Some(Box::new(resource.clone())));
        }

        Ok(None)
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }
}
