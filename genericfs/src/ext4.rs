//! Ext4

use arcbox_ext4::Reader;
use memmap2::Mmap;
use parse::{le_u16, take_arr};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

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

impl GenFS for Ext4F {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {

        let file = f.owned_file();
        let fd = file.as_raw_fd();
        let mut r = Reader::new(&PathBuf::from(format!("/proc/self/fd/{fd}")))?;

        let mut o = Vec::new();
        let mut seen = Vec::new();

        let mut ents = r
            .list_dir("/")?
            .into_iter()
            .map(|x| format!("/{x}"))
            .collect::<Vec<_>>();

        while let Some(e) = ents.pop() {
            if seen.contains(&e) {
                continue;
            }
            seen.push(e.clone());

            let (_, stats) = r.stat_no_follow(&e)?;

            let name = e.rsplit('/').next().unwrap_or(&e).to_string();

            if stats.is_link() {
                o.push(BufGenItm::new_empty(format!("{}.symlink", name)));
            } else {
                if stats.is_dir() {
                    ents.extend(r
                        .list_dir(&e)?
                        .into_iter()
                        .map(|n| format!("{e}/{n}")));
                } else if stats.is_reg() {
                    let data = r.read_file(&e, 0, None)?;
                    o.push(BufGenItm::new(name, data));
                }
            }
        }

        Ok(Self { o, idx: 0 })
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        let (i, _) = take_arr::<1024>(f)?;
        let (i, _) = take_arr::<56>(i)?;
        let (_, magic) = le_u16(i)?;
        Ok(magic == 0xEF53)
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
