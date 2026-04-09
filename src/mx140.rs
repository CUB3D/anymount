//! MX140 (mx140.bin, Firmware for Samsung SCSC wifi, found in A53 5G & A30, cortex-m4)

use parse::{le_u32, take};
use std::collections::HashMap;

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
    tlv::Tlv,
};

pub struct Mx140F {
    pub t: Vec<Tlv>,
    pub idx: usize,
}

impl GenFSProps for Mx140F {
    const FORMAT_NAME: &'static str = "mx140";
}

impl GenFS for Mx140F {
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
            let (i, tag) = le_u32(j)?;
            let (i, len) = le_u32(i)?;
            let (i, data) = take(i, len as usize)?;

            if tag == 0 {
                return Err(anyhow::anyhow!("Found invalid tag 0"));
            }

            j = i;

            o.push(Tlv {
                tag,
                len,
                val: data.to_vec(),
            })
        }

        Ok(Self { t: o, idx: 0 })
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        let mut tag_names = HashMap::new();
        tag_names.insert(0x001, "WLAN_FW");
        tag_names.insert(0x002, "BT_FW");
        tag_names.insert(0x003, "PMU_FW");
        tag_names.insert(0x101, "WLAN_STRINGS");
        tag_names.insert(0x102, "BT_STRINGS");
        tag_names.insert(0x103, "PMU_STRINGS");
        tag_names.insert(0x201, "META_BRANCH");
        tag_names.insert(0x202, "META_BUILD_ID");
        tag_names.insert(0x300, "FW_CMN_HASH");
        tag_names.insert(0x301, "FW_WLAN_HASH");
        tag_names.insert(0x302, "FW_BT_HASH");
        tag_names.insert(0x303, "FW_PMU_HASH");
        tag_names.insert(0x304, "FW_HW_HASH");
        tag_names.insert(0x52444846, "TOTAL_LEN");

        if let Some(t) = self.t.get(self.idx) {
            self.idx += 1;
            let name = tag_names
                .get(&t.tag)
                .map(|c| c.to_string())
                .unwrap_or(format!("{:04x}", t.tag));
            Ok(Some(Box::new(BufGenItm {
                data: t.val.clone(),
                name: format!("{}.bin", name),
                pos: 0,
            })))
        } else {
            Ok(None)
        }
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }
}
