use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};
use memmap2::Mmap;
use std::fmt::{Debug, Formatter};
use std::{fs::File, io::Read};
use tar::Archive;
use tracing::debug;

pub struct TarFile {
    pub tar: ::tar::Archive<File>,
    pub idx: usize,
    pub ents: Vec<BufGenItm>,
}
impl Debug for TarFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

impl GenFSProps for TarFile {
    const FORMAT_NAME: &'static str = "tar";
}
impl GenFS for TarFile {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let mmap = f.mmap;
        let f = f.owned_file();

        let aa = Archive::new(f);
        let mut a = Archive::new(mmap.as_ref());

        //dogshit library
        let mut x = Vec::new();
        for b in a.entries()? {
            let mut b = b?;
            let mut d = Vec::new();
            b.read_to_end(&mut d)?;

            let name = b.path()?.file_name().unwrap().to_string_lossy().to_string();
            debug!("tar: {}", name);
            x.push(BufGenItm::new(name, d));
        }

        Ok(Self {
            tar: aa,
            ents: x,
            idx: 0,
        })
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        let hdr = f
            .get(257..)
            .ok_or_else(|| anyhow::anyhow!("Not enough data"))?;
        let hdr = hdr
            .get(..5)
            .ok_or_else(|| anyhow::anyhow!("Not enough data"))?;

        if hdr != [0x75, 0x73, 0x74, 0x61, 0x72] {
            return Ok(false);
        }

        Ok(true)
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

#[cfg(test)]
pub mod tests {
    use crate::generic_fs::GenFS;
    use crate::mapped_file::MappedFile;

    #[test]
    pub fn basic() {
        let f = MappedFile::open("./tests/empty.tar");
        let fref = f.get_ref();
        super::TarFile::sniff(&fref.mmap).unwrap();
        super::TarFile::try_open_internal(&fref).unwrap();
    }
}
