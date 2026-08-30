//! Unix AR archives
use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};
use ar::Archive;
use memmap2::Mmap;
use std::io::Read;

pub struct UnixArF {
    pub idx: usize,
    pub ents: Vec<BufGenItm>,
}

impl GenFSProps for UnixArF {
    const FORMAT_NAME: &'static str = "unix_ar";
}

impl GenFS for UnixArF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let mut ents = Vec::new();
        let mut archive = Archive::new(f.mmap.as_ref());

        while let Some(entry) = archive.next_entry() {
            let mut entry = entry?;

            let name = String::from_utf8_lossy(entry.header().identifier())
                .trim_end_matches('/')
                .to_string();

            let mut d = Vec::new();
            entry.read_to_end(&mut d)?;

            ents.push(BufGenItm::new(name, d));
        }

        Ok(Self { ents, idx: 0 })
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        Ok(f.get(..8) == Some(b"!<arch>\n"))
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        if let Some(f) = self.ents.get(self.idx) {
            self.idx += 1;
            return Ok(Some(Box::new(f.clone())));
        }

        Ok(None)
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }
}
