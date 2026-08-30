//! QCOW2

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};
use positioned_io::ReadAt;
use positioned_io::Size;
use qcow2::Qcow2;

pub struct Qcow2F {
    pub idx: usize,
    pub o: Vec<BufGenItm>,
}

impl GenFSProps for Qcow2F {
    const FORMAT_NAME: &'static str = "qcow2";
}

impl GenFS for Qcow2F {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let mmap = f.mmap;
        let q = Qcow2::open(&mmap[..])?;
        let r = q.reader()?;
        let mut dat = vec![0u8; r.size()?.unwrap() as _];
        r.read_exact_at(0, &mut dat)?;

        let o = vec![BufGenItm::new("_raw_disk", dat)];

        Ok(Self { o, idx: 0 })
    }

    fn sniff(f: &memmap2::Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        Ok(Qcow2::open(&f[..]).is_ok())
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
