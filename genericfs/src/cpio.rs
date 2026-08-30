use memmap2::Mmap;
use tracing::info;

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};

pub struct CpioFile {
    pub idx: usize,
    pub ents: Vec<BufGenItm>,
}

impl GenFSProps for CpioFile {
    const FORMAT_NAME: &'static str = "cpio";
}

impl GenFS for CpioFile {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let mut ents = Vec::new();
        let mut file = f.owned_file();
        loop {
            // cant use mmap, needs us to handle seeking
            let r = cpio::NewcReader::new(file)?;

            if r.entry().is_trailer() {
                break;
            }

            let name = r.entry().name().to_string();
            let size = r.entry().file_size();

            let mut d = Vec::new();
            file = r.to_writer(&mut d)?;

            if size == 0 {
                info!("Skipping 0 size {name}");
            } else {
                ents.push(BufGenItm {
                    name,
                    data: d,
                    pos: 0,
                });
            }
        }

        Ok(Self { ents, idx: 0 })
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        Ok((f.get(..6) == Some(b"070701")) || (f.get(..6) == Some(b"070702")))
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
