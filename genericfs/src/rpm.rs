//! RPM archives
use std::io::{BufReader, Read};

use memmap2::Mmap;
use rpm::PackageReader;

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};

pub struct RpmF {
    reader: PackageReader,
}

impl GenFSProps for RpmF {
    const FORMAT_NAME: &'static str = "rpm";
}

impl GenFS for RpmF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let reader = PackageReader::parse(BufReader::new(f.owned_file()))?;
        Ok(Self { reader })
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        Ok(f.get(..4) == Some(&[0xed, 0xab, 0xee, 0xdb]))
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        if let Some(mut file) = self.reader.next_file()? {
            let name = file.metadata.path().display().to_string();
            let mut d = Vec::new();
            file.read_to_end(&mut d)?;
            file.finish()?;
            return Ok(Some(Box::new(BufGenItm::new(name, d))));
        }

        Ok(None)
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }
}