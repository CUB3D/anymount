//! XAR archives

use apple_xar::reader::XarReader;
use apple_xar::table_of_contents::FileType;
use memmap2::Mmap;
use std::fs::File;

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};

pub struct XarF {
    reader: XarReader<File>,
    files: Vec<(String, apple_xar::table_of_contents::File)>,
    idx: usize,
}

impl GenFSProps for XarF {
    const FORMAT_NAME: &'static str = "xar";
}

impl GenFS for XarF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let reader = XarReader::new(f.owned_file())?;
        let files = reader.files()?;

        Ok(Self {
            reader,
            files,
            idx: 0,
        })
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        Ok(f.get(..4) == Some(b"xar!"))
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        while let Some((name, file)) = self.files.get(self.idx) {
            self.idx += 1;

            if matches!(file.file_type, FileType::File) {
                let mut d = Vec::new();
                self.reader.write_file_data_decoded_from_file(file, &mut d)?;
                return Ok(Some(Box::new(BufGenItm {
                    name: name.clone(),
                    data: d,
                    pos: 0,
                })));
            }
        }

        Ok(None)
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }
}