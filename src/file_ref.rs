use memmap2::Mmap;
use std::fs::File;

pub struct FileRef<'a> {
    pub mmap: &'a Mmap,
    pub file: &'a File,
}

impl FileRef<'_> {
    pub fn owned_file(&self) -> File {
        self.file.try_clone().unwrap()
    }

    pub fn owned_map(&self) -> Mmap {
        unsafe { Mmap::map(&self.owned_file()).unwrap() }
    }
}
