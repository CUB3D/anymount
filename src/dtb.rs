//! Devicetree

use dtb::Reader;
use memmap2::Mmap;

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};

pub struct DtbF {
    pub idx: usize,
    pub o: Vec<BufGenItm>,
}

impl GenFSProps for DtbF {
    const FORMAT_NAME: &'static str = "dtb";
}

impl GenFS for DtbF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let mut o = Vec::new();

        let dt = Reader::read(f.mmap.as_ref()).unwrap();

        let mut out = String::new();

        for entry in dt.reserved_mem_entries() {
            out.push_str(&format!(
                "reserved: {:?}, {:?}\n",
                entry.address, entry.size
            ));
        }

        o.push(BufGenItm::new("_device_tree.txt", out.as_bytes().to_vec()));

        Ok(Self { o, idx: 0 })
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        Ok(Reader::read(f.as_ref()).is_ok())
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
