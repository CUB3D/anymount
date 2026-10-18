use crate::file_ref::FileRef;
use memmap2::Mmap;
use std::fs::{File, OpenOptions};
use std::path::Path;
use anyhow::Context;

pub struct MappedFile {
    fmap_file: File,
    fmap: Mmap,
}

impl MappedFile {
    pub fn open<T: AsRef<Path>>(p: T) -> anyhow::Result<Self> {
        let p = p.as_ref();
        let fmap_file = OpenOptions::new()
            .read(true)
            .open(p)
            .expect("Input file not found");
        Self::from_file(fmap_file)
    }

    pub fn from_file(fmap_file: File) -> anyhow::Result<Self> {
        let fmap = unsafe { Mmap::map(&fmap_file) }.context("Failed to mmap file")?;
        Ok(Self { fmap_file, fmap })
    }

    pub fn get_ref(&self) -> FileRef<'_> {
        FileRef {
            mmap: &self.fmap,
            file: &self.fmap_file,
        }
    }
}
