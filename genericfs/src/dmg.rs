//! Apple DMG

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};

pub struct DmgF {
    pub idx: usize,
    pub o: Vec<BufGenItm>,
}

impl GenFSProps for DmgF {
    const FORMAT_NAME: &'static str = "dmg";
}

impl GenFS for DmgF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let mut archive = udif::DmgReader::new(f.owned_file())?;
        
        let mut o = Vec::new();
        for p in archive.partitions().to_vec() {
            let data = archive.decompress_partition(p.id)?;
            o.push(BufGenItm::new(p.name.clone(), data));
        }

        Ok(Self { o, idx: 0 })
    }

    fn sniff(f: &memmap2::Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        let start = f.len() - 512;
        return Ok(f.get(start..(start+4)) == Some(b"koly"))
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