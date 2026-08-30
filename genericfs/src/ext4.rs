//! Ext4

use anyhow::Context;
use ext4_view::{Ext4, Ext4Read};
use memmap2::Mmap;
use std::error::Error;

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};

pub struct Ext4F {
    pub idx: usize,
    pub o: Vec<BufGenItm>,
}

impl GenFSProps for Ext4F {
    const FORMAT_NAME: &'static str = "ext4";
}

struct MmapWrapper {
    slc: Mmap,
}
impl Ext4Read for MmapWrapper {
    fn read(
        &mut self,
        start_byte: u64,
        dst: &mut [u8],
    ) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
        dst.copy_from_slice(
            self.slc
                .get(start_byte as usize..)
                .context("Start oob")?
                .get(..dst.len())
                .context("end oob")?,
        );
        Ok(())
    }
}

impl GenFS for Ext4F {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let mmap = f.owned_map();

        let ext = Ext4::load(Box::new(MmapWrapper { slc: mmap }))?;

        let mut entries = Vec::new();
        let mut seen = Vec::new();
        for c in ext.read_dir("/")? {
            let c = c?;
            entries.push(c);
        }

        let mut o = Vec::new();

        while let Some(entry) = entries.pop() {
            if seen.contains(&entry.path()) {
                continue;
            }
            seen.push(entry.path());

            let ft = entry.file_type()?;

            if ft.is_symlink()
                || ft.is_socket()
                || ft.is_fifo()
                || ft.is_char_dev()
                || ft.is_block_dev()
            {
                continue;
            }

            if ft.is_dir() {
                for c in ext.read_dir(&entry.path())? {
                    let c = c?;
                    entries.push(c);
                }
            }

            if ft.is_regular_file() {
                let data = ext.read(&entry.path())?;
                o.push(BufGenItm::new(entry.file_name().as_str()?, data));
            }
        }

        Ok(Self { o, idx: 0 })
    }

    fn sniff(_f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        //TODOs
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
