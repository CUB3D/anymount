//! APFS

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};
use std::io::{Read, Seek, SeekFrom};

pub struct ApfsF {
    pub idx: usize,
    pub o: Vec<BufGenItm>,
}

impl GenFSProps for ApfsF {
    const FORMAT_NAME: &'static str = "apfs";
}

fn walk<R: Read + Seek>(
    reader: &mut R,
    volume: &apfs_core::volume::ApfsVolume,
    parent_oid: u64,
    block_size: usize,
    seen: &mut Vec<u64>,
    o: &mut Vec<BufGenItm>,
) -> anyhow::Result<()> {
    if seen.contains(&parent_oid) {
        return Ok(());
    }
    seen.push(parent_oid);

    let entries = apfs_core::dir::list_dir(reader, volume, parent_oid, block_size)?;
    for entry in entries {
        let inode = match apfs_core::dir::load_inode(reader, volume, entry.file_id, block_size) {
            Ok(i) => i,
            Err(_) => continue,
        };

        let is_dir = inode.mode & 0o040000 == 0o040000;
        let is_symlink = inode.mode & 0o120000 == 0o120000;
        let is_reg = inode.mode & 0o170000 == 0o100000;

        if is_symlink {
            o.push(BufGenItm::new_empty(format!("{}.symlink", entry.name)));
        } else if is_reg {
            if let Ok(data) =
                apfs_core::extent::read_data(reader, volume, &inode, block_size)
            {
                o.push(BufGenItm::new(entry.name, data));
            }
        } else if is_dir && entry.name != "." && entry.name != ".." {
            walk(reader, volume, entry.file_id, block_size, seen, o)?;
        }
    }

    Ok(())
}

impl GenFS for ApfsF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let mut container = apfs_core::ApfsContainer::open(f.owned_file())?;
        let block_size = container.superblock().block_size as usize;
        let addrs = container.volume_superblock_addrs()?;
        let mut reader = container.into_reader();

        let mut o = Vec::new();

        // Find superblock
        let mut volume = None;
        for addr in addrs {
            let mut block = vec![0u8; block_size as usize];
            reader.seek(SeekFrom::Start(addr * block_size as u64))?;
            reader.read_exact(&mut block)?;
    

            if let Ok(v) = apfs_core::volume::ApfsVolume::parse(&block) {
                volume = Some(v);
                break;
            }
        }
        let volume = volume.unwrap();

        let mut seen = Vec::new();
        walk(
            &mut reader,
            &volume,
            apfs_core::dir::ROOT_DIR_INO_NUM,
            block_size,
            &mut seen,
            &mut o,
        )?;

        Ok(Self { o, idx: 0 })
    }

    fn sniff(f: &memmap2::Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        Ok(f.get(32..36) == Some(b"NXSB"))
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