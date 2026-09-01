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

#[derive(Copy, Clone, Debug)]
pub enum ZipEntry {
    File(usize),
    Extra(usize),
    Comment(usize),
    Encrypted(usize),
    Symlink(usize),
}

#[derive(Debug)]
pub struct ZipFile {
    pub zip: ::zip::ZipArchive<BufReader<File>>,
    pub entries: Vec<ZipEntry>,
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
        let mut z = zip::ZipArchive::with_config(c, BufReader::new(f))?;

        let mut entries = Vec::new();
        let mut idx = 0;
        while let Ok(f) = z.by_index(idx) {
            if f.encrypted() {
                entries.push(ZipEntry::Encrypted(idx));
            } else if f.is_symlink() {
                entries.push(ZipEntry::Symlink(idx));
            } else {
                // Skip non-encrypted dirs
                if !f.is_dir() {
                    idx += 1;
                    continue;
                }

                entries.push(ZipEntry::File(idx));

                if !f.comment().is_empty() {
                    entries.push(ZipEntry::Comment(idx));
                }

                if f.extra_data().is_some() {
                    entries.push(ZipEntry::Extra(idx));
                }
            }
        }

        Ok(Self {
            zip: z,
            idx: 0,
            entries,
        })
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        Ok((f.get(..4) == Some(&[0x50, 0x4b, 0x03, 0x04]))
            || (f.get(..4) == Some(&[0x50, 0x4b, 0x05, 0x06]))
            || (f.get(..4) == Some(&[0x50, 0x4b, 0x07, 0x08])))
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        if let Some(&ent) = self.entries.get(self.idx) {
            self.idx += 1;

            match ent {
                ZipEntry::Encrypted(id) => {
                    let f = self.zip.by_index(id)?;
                    warn!("File: {} is encrypted, skipping", f.name());
                    return Ok(Some(Box::new(BufGenItm::new_empty(format!("{}.encrypted", f.name())))));
                }
                ZipEntry::Symlink(id) => {
                    let f = self.zip.by_index(id)?;
                    warn!("File: {} is symlink, skipping", f.name());
                    return Ok(Some(Box::new(BufGenItm::new_empty(format!("{}.symlink", f.name())))));
                }
                ZipEntry::File(id) => {
                    let mut f = self.zip.by_index(id)?;
                    let mut d = Vec::with_capacity(f.size() as usize);
                    f.read_to_end(&mut d)?;
                    return Ok(Some(Box::new(BufGenItm::new(f.name(), d))));
                }
                ZipEntry::Extra(id) => {
                    let f = self.zip.by_index(id)?;
                    return Ok(Some(Box::new(BufGenItm::new(
                        format!("{}.extra_data", f.name()),
                        f.extra_data().unwrap().to_vec(),
                    ))));
                }
                ZipEntry::Comment(id) => {
                    let f = self.zip.by_index(id)?;
                    return Ok(Some(Box::new(BufGenItm::new(
                        format!("{}.comment", f.name()),
                        f.comment().as_bytes().to_vec(),
                    ))));
                }
            }
        } else {
            Ok(None)
        }
    }

    fn name(&self) -> &str {
        "ZIP"
    }
}
