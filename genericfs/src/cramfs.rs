//! CramFS filesystem

use std::fs::File;
use cramfs::{Cramfs, DirEntry};
use memmap2::Mmap;
use crate::file_ref::FileRef;
use crate::gen_item::{BufGenItm, GenItem};
use crate::generic_fs::GenFS;
use crate::generic_fs_props::GenFSProps;

pub struct CramfsF {
    idx: usize,
    o: Vec<BufGenItm>,
}

impl GenFSProps for CramfsF {
    const FORMAT_NAME: &'static str = "cramfs";
}

impl GenFS for CramfsF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let cram = Cramfs::from_reader(f.owned_file())?;

        fn recurse_dir(e: &DirEntry<File>, path: &str, out: &mut Vec<BufGenItm>) -> anyhow::Result<()> {
            for f in e.read_dir()? {
                let mut f = f?;

                let name = f.path().to_string_lossy().to_string();
                let path = if path.is_empty() {
                    name
                } else {
                    format!("{path}/{name}")
                };

                if f.is_dir() {
                    recurse_dir(&f, &path, out)?;
                } else if f.is_symlink() {
                    out.push(BufGenItm::new_empty(format!("{path}")));
                } else {
                    let mut d = Vec::new();
                    f.read_file(&mut d)?;
                    out.push(BufGenItm::new(path, d));
                }
            }

            Ok(())
        }

        let mut o = Vec::new();
        recurse_dir(&cram.root()?, "", &mut o)?;

        Ok(Self { o, idx: 0 })
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        for off in 0..512 {
            if f.get(off..(off + 4)) == Some(&[0x45, 0x3d, 0xcd, 0x28]) {
                return Ok(true);
            }
        }
        Ok(false)
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
