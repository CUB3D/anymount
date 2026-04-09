use anyhow::Context;
use decode_raw::{Entry, EntryValue, ParseConfig};
use memmap2::Mmap;

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};

pub struct PBufF {
    pub idx: usize,
    pub pbuf: Vec<Entry>,
}

impl GenFSProps for PBufF {
    const FORMAT_NAME: &'static str = "protobuf";
}

impl GenFS for PBufF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let pbuf = decode_raw::try_parse_entries(f.mmap, ParseConfig::default())
            .context("Not protobuf")?;

        Ok(Self { idx: 0, pbuf })
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        let is_pbuf = decode_raw::try_parse_entries(f, ParseConfig::default()).is_some();

        Ok(is_pbuf)
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        if self.idx == 1 {
            return Ok(None);
        }

        let mut s = String::new();
        for e in &self.pbuf {
            let pth = if let Ok(ss) = String::from_utf8(e.path.iter().map(|c| *c as u8).collect()) {
                ss
            } else {
                format!("{:?}", e.path)
            };

            let vv = if let EntryValue::Bytes(b) = &e.value {
                if let Ok(ss) = String::from_utf8(b.to_vec()) {
                    ss
                } else {
                    format!("{:?}", e.value)
                }
            } else {
                format!("{:?}", e.value)
            };

            s.push_str(&format!("pth: {:?}\n", pth));
            s.push_str(&format!("value: {:?}\n", vv));
        }

        self.idx += 1;
        Ok(Some(Box::new(BufGenItm {
            name: "protobuf_data".to_string(),
            data: s.as_bytes().to_vec(),
            pos: 0,
        })))
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }

    fn single_output(&self) -> bool {
        true
    }
}
