//! Android boot image

use abootimg_oxide::{Header, HeaderV0Versioned};
use memmap2::Mmap;

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};

pub struct AbootimgF {
    pub idx: usize,
    pub o: Vec<BufGenItm>,
}

impl GenFSProps for AbootimgF {
    const FORMAT_NAME: &'static str = "abootimg";
}

impl GenFS for AbootimgF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let mmap = f.mmap;
        let hdr = abootimg_oxide::Header::parse(&mut std::io::Cursor::new(mmap.as_ref()))?;

        let mut o = Vec::new();
        o.push(BufGenItm {
            pos: 0,
            name: "_header_details.txt".to_string(),
            data: format!("{:?}", hdr).to_string().as_bytes().to_vec(),
        });
        o.push(BufGenItm {
            pos: 0,
            name: "cmdline.txt".to_string(),
            data: hdr.cmdline().to_vec(),
        });
        o.push(BufGenItm {
            pos: 0,
            name: "kernel".to_string(),
            data: mmap.as_ref()[hdr.kernel_position()..][..hdr.kernel_size() as _].to_vec(),
        });
        o.push(BufGenItm {
            pos: 0,
            name: "ramdisk".to_string(),
            data: mmap.as_ref()[hdr.ramdisk_position()..][..hdr.ramdisk_size() as _].to_vec(),
        });

        if let Header::V0(v0) = hdr {
            if v0.second_bootloader_size != 0 {
                o.push(BufGenItm {
                    pos: 0,
                    name: "second_bootloader".to_string(),
                    data: mmap.as_ref()[v0.second_bootloader_addr as _..]
                        [..v0.second_bootloader_size as _]
                        .to_vec(),
                });
            }

            match v0.versioned {
                HeaderV0Versioned::V0 => {}
                HeaderV0Versioned::V1 {
                    recovery_dtbo_addr,
                    recovery_dtbo_size,
                } => {
                    if recovery_dtbo_size != 0 {
                        o.push(BufGenItm {
                            pos: 0,
                            name: "recovery_dtbo".to_string(),
                            data: mmap.as_ref()[recovery_dtbo_addr as _..]
                                [..recovery_dtbo_size as _]
                                .to_vec(),
                        });
                    }
                }
                HeaderV0Versioned::V2 {
                    recovery_dtbo_addr,
                    recovery_dtbo_size,
                    dtb_addr: _,
                    dtb_size,
                } => {
                    if recovery_dtbo_size != 0 {
                        o.push(BufGenItm {
                            pos: 0,
                            name: "recovey_dtbo".to_string(),
                            data: mmap.as_ref()[recovery_dtbo_addr as _..]
                                [..recovery_dtbo_size as _]
                                .to_vec(),
                        });
                    }
                    if let Some(dtb_pos) = v0.dtb_position() {
                        o.push(BufGenItm {
                            pos: 0,
                            name: "dtb".to_string(),
                            data: mmap.as_ref()[dtb_pos as _..][..dtb_size as _].to_vec(),
                        });
                    }
                }
            }
        }

        Ok(Self { o, idx: 0 })
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        Ok(abootimg_oxide::Header::parse(&mut std::io::Cursor::new(f.as_ref())).is_ok())
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
