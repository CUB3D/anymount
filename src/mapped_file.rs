use crate::file_ref::FileRef;
use memmap2::Mmap;
use std::fs::{File, OpenOptions};
use std::path::Path;

pub struct MappedFile {
    fmap_file: File,
    fmap: Mmap,
}

impl MappedFile {
    pub fn open<T: AsRef<Path>>(p: T) -> Self {
        let p = p.as_ref();
        let fmap_file = OpenOptions::new()
            .read(true)
            .open(p)
            .expect("Input file not found");
        let fmap = unsafe { Mmap::map(&fmap_file) }.unwrap();

        Self { fmap_file, fmap }
    }

    pub fn get_ref(&self) -> FileRef<'_> {
        FileRef {
            mmap: &self.fmap,
            file: &self.fmap_file,
        }
    }
}
