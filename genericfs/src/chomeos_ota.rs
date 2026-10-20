//! ChromeOS OTA files

use std::io::{Cursor, Read};

use crate::file_ref::FileRef;
use crate::gen_item::BufGenItm;
use crate::generic_fs_props::GenFSProps;
use crate::{gen_item::GenItem, generic_fs::GenFS};
use anyhow::Context;
use memmap2::Mmap;
use parse::{ParseError, be_u32, le_u32, take};
use prost::Message;
use xz::read::XzDecoder;

pub mod chromeos_update_engine {
    include!(concat!(env!("OUT_DIR"), "/chromeos_update_engine.rs"));
}

///See https://github.com/sebanc/chromeos-ota-extract/blob/master/extract_android_ota_payload.py
pub struct ChromeosOTAF {
    pub idx: usize,
    pub ents: Vec<BufGenItm>,
}

impl GenFSProps for ChromeosOTAF {
    const FORMAT_NAME: &'static str = "ChromeOS OTA";
}

impl GenFS for ChromeosOTAF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let (i, _magic) = le_u32(&f.mmap)?;
        let (i, version) = be_u64(i)?;
        let (i, manifest_len) = be_u64(i)?;

        if version != 2 {
            return Err(anyhow::anyhow!("Only version 2 of Chomeos OTA is supported so far, sorry"));
        }

        //TODO: ?
        let (i, sig_len) = be_u32(i);

        let mut ents = Vec::new();

        let (i, raw_manifest) = take(i, manifest_len as usize)?;
        let dam = chromeos_update_engine::DeltaArchiveManifest::decode(&mut Cursor::new(raw_manifest))?;
        // println!("{:?}", dam);
        ents.push(BufGenItm::new("_raw_manifest.txt", format!("{dam:?}").into_bytes()));

        let (_, meta_sig) = take(i, sig_len as usize)?;
        let sig = chromeos_update_engine::Signatures::decode(&mut Cursor::new(meta_sig))?;
        // println!("{:?}", sig);
        ents.push(BufGenItm::new("_sig.txt", format!("{sig:?}").into_bytes()));

        for p in dam.partitions {
            //println!("part - {}", p.partition_name);

            let mut part_data = Vec::new();

            let mut tmp_dat = Vec::new();

            for op in p.operations {
                let e = op.dst_extents.first().unwrap();
                let dat = &f.mmap[(24 + manifest_len as usize + sig_len as usize)..][op.data_offset() as usize..][..op.data_length() as usize];

                tmp_dat.clear();

                part_data.resize((e.start_block() + e.num_blocks()) as usize * 0x1000, 0);

                match op.r#type() {
                    chromeos_update_engine::install_operation::Type::Replace => {
                        part_data[(e.start_block() as usize * 0x1000)..][..dat.len()].copy_from_slice(&dat);
                    }
                    chromeos_update_engine::install_operation::Type::ReplaceXz => {
                        let mut f = XzDecoder::new(dat);
                        f.read_to_end(&mut tmp_dat).context("failed to xz decompress")?;

                        //TODO: sha256
                        
                        part_data[(e.start_block() as usize * 0x1000)..][..tmp_dat.len()].copy_from_slice(&tmp_dat);
                    }
                    chromeos_update_engine::install_operation::Type::ReplaceBz => {
                        let mut f = bzip2::read::BzDecoder::new(dat);
                        f.read_to_end(&mut tmp_dat)?;

                        //TODO: sha256
                        
                        part_data[(e.start_block() as usize * 0x1000)..][..tmp_dat.len()].copy_from_slice(&tmp_dat);
                    }
                    chromeos_update_engine::install_operation::Type::Zero => {}
                    _ => {
                        println!("Unhandled operation type: {:?}", op.r#type())
                    }
                }
            }

            ents.push(BufGenItm::new(format!("{}.img", p.partition_name), part_data));
        }


        Ok(Self {
            idx: 0,
            ents,
        })
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        Ok(f.get(..4) == Some(b"CrAU"))
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

//TODO:
pub fn be_u64(i: &[u8]) -> Result<(&[u8], u64), ParseError> {
    if i.len() < 8 {
        return Err(ParseError::NoData);
    }
    Ok((&i[8..], u64::from_be_bytes((&i[..8]).try_into().unwrap())))
}
