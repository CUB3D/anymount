use std::{
    fs::File,
    io::{BufReader, Read},
};

use memmap2::Mmap;
use tracing::warn;
use zip::read::Config;

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};

#[derive(Debug)]
pub struct ZipFile {
    pub zip: ::zip::ZipArchive<BufReader<File>>,
    pub idx: usize,
}

impl GenFSProps for ZipFile {
    const FORMAT_NAME: &'static str = "zip";
}

impl GenFS for ZipFile {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let f = f.owned_file();
        let c = Config::default();
        let z = zip::ZipArchive::with_config(c, BufReader::new(f))?;
        Ok(Self { zip: z, idx: 0 })
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        Ok((f.get(..4) == Some(&[0x50, 0x4b, 0x03, 0x04])) || (f.get(..4) == Some(&[0x50, 0x4b, 0x05, 0x06])) || (f.get(..4) == Some(&[0x50, 0x4b, 0x07, 0x08])))
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        let mut f = {
            let mut f = self.zip.by_index(self.idx)?;

            // Find next non dir
            loop {
                // Skip encrypted
                if f.encrypted() {
                    warn!("File: {} is encrypted, skipping", f.name());
                } else if f.is_symlink() {
                    warn!("File: {} is symlink, skipping", f.name());
                } else {
                    // Skip non-encrypted dirs
                    if !f.is_dir() {
                        break;
                    }
                }
                self.idx += 1;
                drop(f);
                f = self.zip.by_index(self.idx)?
            }

            f
        };

        if let Some(extra) = f.extra_data() {
            warn!("File: {} has extra data ({} bytes)", f.name(), extra.len());
        }

        if !f.comment().is_empty() {
            warn!("File: {} has comment", f.name());
        }

        self.idx += 1;
        let mut d = Vec::with_capacity(f.size() as usize);
        f.read_to_end(&mut d)?;
        Ok(Some(Box::new(BufGenItm {
            name: f.name().to_string(),
            data: d,
            pos: 0,
        })))
    }

    fn name(&self) -> &str {
        "ZIP"
    }
}
