use parse::{le_u32, take};

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};

#[derive(Debug)]
pub struct Tlv {
    pub tag: u32,
    pub len: u32,
    pub val: Vec<u8>,
}

pub struct TlvF {
    pub t: Vec<Tlv>,
    pub idx: usize,
}

impl GenFSProps for TlvF {
    const FORMAT_NAME: &'static str = "tlv";
}

impl GenFS for TlvF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let v = f.mmap;

        let (mut j, _i) = take(v, 8)?;
        let mut o = Vec::new();
        loop {
            if j.is_empty() || o.len() > 100 {
                break;
            }

            let (i, t) = le_u32(j)?;
            let (i, l) = le_u32(i)?;
            let (i, d) = take(i, l as usize)?;
            j = i;

            o.push(Tlv {
                tag: t,
                len: l,
                val: d.to_vec(),
            })
        }

        Ok(TlvF { t: o, idx: 0 })
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        if let Some(t) = self.t.get(self.idx) {
            self.idx += 1;
            Ok(Some(Box::new(BufGenItm::new(
                format!("tlv_{:04x}", t.tag),
                t.val.clone(),
            ))))
        } else {
            Ok(None)
        }
    }

    fn name(&self) -> &str {
        "TLV"
    }
}
