//! Enhanced Read Only Filesystem
//! https://erofs.docs.kernel.org/en/latest/core_ondisk.html

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};
use memmap2::Mmap;
use parse::{le_u16, le_u32, le_u64, ne_u8, take};

pub struct ErofsF {
    pub idx: usize,
    pub o: Vec<BufGenItm>,
}

impl GenFSProps for ErofsF {
    const FORMAT_NAME: &'static str = "erofs";
}

impl GenFS for ErofsF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let mmap = f.mmap;

        let (i, _magic) = le_u32(&mmap[1024..])?;
        let (i, _chk) = le_u32(i)?;
        let (i, _feat) = le_u32(i)?;
        let (i, _blkszbits) = ne_u8(i)?;
        let (i, _sb_extslots) = ne_u8(i)?;
        let (i, _root_nid) = le_u16(i)?;
        let (i, _inos) = le_u64(i)?;
        let (i, _build_time) = le_u64(i)?;
        let (i, _build_time_ns) = le_u32(i)?;
        let (i, _blocks) = le_u32(i)?;
        let (i, _meta_blkaddr) = le_u32(i)?;
        let (i, _xattr_blkaddr) = le_u32(i)?;
        let (i, _uuid) = take(i, 16)?;
        let (i, _volname) = take(i, 16)?;
        let (i, _feat_incompat) = le_u32(i)?;
        let (i, _avail_compr_algs) = le_u16(i)?;
        let (i, _lz4_max_dist) = le_u16(i)?;
        let (i, _extra_dev) = le_u16(i)?;
        let (i, _devt_slotoff) = le_u16(i)?;
        let (i, _dirblkbits) = ne_u8(i)?;
        let (i, _xattr_prefix_cnd) = ne_u8(i)?;
        let (i, _xattr_prefix_start) = le_u32(i)?;
        let (i, _packed_nid) = le_u64(i)?;
        let (i, _xattr_filter_reserved) = ne_u8(i)?;
        let (_i, _reserve) = take(i, 23)?;

        let blksz = 2u32.pow(_blkszbits as u32);
        let root_inode = &mmap[(_meta_blkaddr * blksz + 32 * _root_nid as u32) as usize..];

        let (i, _i_fmt) = le_u16(root_inode)?;

        if (_i_fmt & 1) == 0 {
            let (i, _i_xattr_icount) = le_u16(i)?;
            let (i, _i_mode) = le_u16(i)?;
            let (i, _i_nlink) = le_u16(i)?;
            let (i, _i_size) = le_u32(i)?;
            let (i, _i_reserved) = le_u32(i)?;
            let (i, _i_u) = le_u32(i)?;
            let (i, _i_ino) = le_u32(i)?;
            let (i, _i_uid) = le_u16(i)?;
            let (i, _i_gid) = le_u16(i)?;
            let (i, _i_reserved2) = le_u32(i)?;

            let is_dir = _i_mode & 0o170000 == 0o40000;

            let mut inode_data = Vec::new();

            match _i_fmt >> 1 {
                // EROFS_INODE_FLAT_INLINE (2)
                0b10 => {
                    if _i_u != 0xffffffff && _i_size < blksz {
                        let first = &mmap[_i_u as usize..];
                        inode_data.extend_from_slice(first);
                    }

                    inode_data.extend_from_slice(&i[.._i_size as usize]);
                }
                _ => panic!(),
            }

            println!("{_i_fmt:b} {_i_mode:o} {is_dir} {_i_size} {_i_u:x}");
            // panic!("{:X?}", inode_data);

            let (i, _nid) = le_u64(&inode_data)?;
            let (i, _nameoff) = le_u16(i)?;
            let (i, _ft) = ne_u8(i)?;
            let (_i, _reseve) = ne_u8(i)?;

            panic!("{:X?}", _nameoff);
        } else {
            panic!()
        }

        // Ok(Self {
        //     idx: 0,
        //     o: Vec::new(),
        // })
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        let hdr = f
            .get(1024..)
            .ok_or_else(|| anyhow::anyhow!("Not enough data"))?;

        let hdr = hdr
            .get(..4)
            .ok_or_else(|| anyhow::anyhow!("Not enough data"))?;

        if hdr != 0xE0F5E1E2_u32.to_le_bytes() {
            return Ok(false);
        }

        Ok(true)
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
