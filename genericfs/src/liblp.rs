use crate::file_ref::FileRef;
use crate::gen_item::{BufGenItm, GenItem};
use crate::generic_fs::GenFS;
use crate::generic_fs_props::GenFSProps;
use anyhow::anyhow;
use memmap2::Mmap;
use parse::{le_u16, le_u32, le_u64, take};
use tracing::{debug, warn};

pub struct LibLPf {
    pub idx: usize,
    pub partitions: Vec<Partition>,
    pub extents: Vec<Extent>,
    pub map: Mmap,
}

impl GenFSProps for LibLPf {
    const FORMAT_NAME: &'static str = "lpf";
}

const LP_PARTITION_RESERVED_BYTES: usize = 0x1000;
const LP_METADATA_GEOMETRY_SIZE: usize = 0x1000;

#[derive(Debug)]
pub struct Partition {
    name: String,
    first_extent_index: u32,
}

#[derive(Debug)]
pub struct Extent {
    num_sectors: u64,
    target_data: u64,
    pub blockdev: u32,
}

#[derive(Debug)]
pub struct BlkDev {
    pub name: String,
    pub first_sector: u64,
    pub size: u64,
}

impl GenFS for LibLPf {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let v = f.owned_map();

        let i = &v[LP_PARTITION_RESERVED_BYTES..];

        let (i, magic) = le_u32(i)?;

        if magic != 0x616c4467 {
            return Err(anyhow!("magic mismatch"));
        }

        // primary geom
        let (i, _size) = le_u32(i)?;
        let (i, _chk) = le_u32(i)?;
        let (i, _meta_max_sz) = le_u32(i)?;
        let (i, _slot_cnt) = le_u32(i)?;
        let (i, _logcal_blk_sz) = le_u32(i)?;
        let _i = i;

        // ignore secondary geom at 0x2000

        //TODO: all slots

        #[expect(clippy::erasing_op)]
        let metadata_offset = LP_PARTITION_RESERVED_BYTES
            + (LP_METADATA_GEOMETRY_SIZE * 2)
            + (_meta_max_sz * 0) as usize;

        let i = &v[metadata_offset..];
        let (i, magic) = le_u32(i)?;
        if magic != 0x414C5030 {
            return Err(anyhow!("magic mismatch"));
        }
        let (i, _major_version) = le_u16(i)?;
        let (i, _minor_version) = le_u16(i)?;
        let (i, _header_size) = le_u32(i)?;
        let (i, _chk) = take(i, 32)?;
        let (i, _tables_size) = le_u32(i)?;
        let (i, _tables_checksum) = take(i, 32)?;
        //LpMetadataTableDescriptor partitions
        let (i, _p_offset) = le_u32(i)?;
        let (i, _p_num_entries) = le_u32(i)?;
        let (i, _p_entry_size) = le_u32(i)?;
        //extents
        let (i, _e_offset) = le_u32(i)?;
        let (i, _e_num_entries) = le_u32(i)?;
        let (i, _e_entry_size) = le_u32(i)?;
        //groups
        let (i, _g_offset) = le_u32(i)?;
        let (i, _g_num_entries) = le_u32(i)?;
        let (i, _g_entry_size) = le_u32(i)?;
        //block_devices
        let (i, _b_offset) = le_u32(i)?;
        let (i, _b_num_entries) = le_u32(i)?;
        let (i, _b_entry_size) = le_u32(i)?;
        let _i = i;

        let meta_header_end = metadata_offset + _header_size as usize;

        let mut partitions = Vec::new();

        let partition_tbl_start = meta_header_end + _p_offset as usize;
        for pi in 0.._p_num_entries as usize {
            let i = &v[partition_tbl_start..][_p_entry_size as usize * pi..];
            let (i, name) = take(i, 36)?;
            let (i, _attributes) = le_u32(i)?;
            let (i, first_extent_index) = le_u32(i)?;
            let (i, _num_extents) = le_u32(i)?;
            let (i, _group_index) = le_u32(i)?;
            let _i = i;

            let name_end = name.iter().position(|c| *c == 0).unwrap_or(name.len());
            let name = &name[..name_end];
            let name = String::from_utf8(name.to_vec())?;

            debug!(
                "{:?} a={_attributes:x} fe={first_extent_index} ne={_num_extents} gi={_group_index}",
                name
            );

            // just skip if it has no data
            if _num_extents == 0 {
                continue;
            }

            if _group_index != 1 {
                warn!("Bad group idx");
                continue;
            }

            assert_eq!(_group_index, 1);
            assert_eq!(_num_extents, 1);
            assert_eq!(_attributes, 1);

            partitions.push(Partition {
                name,
                first_extent_index,
            })
        }

        debug!("partitions: {:?}", partitions);

        let mut extents = Vec::new();

        let ext_tbl_start = meta_header_end + _e_offset as usize;
        for ei in 0.._e_num_entries as usize {
            let i = &v[ext_tbl_start..][_e_entry_size as usize * ei..];
            let (i, num_sectors) = le_u64(i)?;
            let (i, _target_type) = le_u32(i)?;
            let (i, target_data) = le_u64(i)?;
            let (i, target_source) = le_u32(i)?;
            let _i = i;
            debug!("ns={num_sectors} tt={_target_type} data={target_data} ts={target_source}");

            assert_eq!(target_source, 0); // one blkdev
            assert_eq!(_target_type, 0); // linear dev

            extents.push(Extent {
                target_data,
                num_sectors,
                blockdev: target_source,
            })
        }

        debug!("extents: {:?}", extents);

        let _grp_tbl_start = meta_header_end + _g_offset as usize;

        let mut blk_devs = Vec::new();

        let blockdev_tbl_start = meta_header_end + _b_offset as usize;
        for bi in 0.._b_num_entries as usize {
            let i = &v[blockdev_tbl_start..][_b_entry_size as usize * bi..];
            let (i, first_logical_sector) = le_u64(i)?;
            let (i, _alignment) = le_u32(i)?;
            let (i, _alignment_offset) = le_u32(i)?;
            let (i, size) = le_u64(i)?;
            let (i, name) = take(i, 36)?;
            let (i, _flags) = le_u32(i)?;
            let _i = i;

            let name_end = name.iter().position(|c| *c == 0).unwrap_or(name.len());
            let name = &name[..name_end];
            let name = String::from_utf8(name.to_vec())?;

            assert_eq!(_flags, 0);
            assert_eq!(_alignment_offset, 0);

            debug!(
                "a={:x} a={_alignment} ao={_alignment_offset} s={size} {:?} ",
                first_logical_sector * 512,
                name
            );

            blk_devs.push(BlkDev {
                first_sector: first_logical_sector,
                size,
                name,
            });
        }

        debug!("blk_devs: {:?}", blk_devs);

        Ok(LibLPf {
            idx: 0,
            partitions,
            extents,
            map: v,
        })
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        let hdr = f
            .get(LP_PARTITION_RESERVED_BYTES..)
            .ok_or_else(|| anyhow::anyhow!("Not enough data"))?;

        let hdr = hdr
            .get(..4)
            .ok_or_else(|| anyhow::anyhow!("Not enough data"))?;

        if hdr != 0x616c4467u32.to_le_bytes() {
            return Ok(false);
        }

        Ok(true)
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        let p = &mut self.partitions.get(self.idx);

        match p {
            Some(p) => {
                self.idx += 1;
                let e = &self.extents[p.first_extent_index as usize];
                let data =
                    &self.map[e.target_data as usize * 512..][..e.num_sectors as usize * 512];
                Ok(Some(Box::new(BufGenItm {
                    name: format!("partition-{}", p.name),
                    data: data.to_vec(),
                    pos: 0,
                })))
            }
            None => Ok(None),
        }
    }

    fn name(&self) -> &str {
        "liblp"
    }
}
