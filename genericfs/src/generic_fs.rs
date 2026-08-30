use crate::generic_fs_props::GenFSProps;
use memmap2::Mmap;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use tracing::warn;

use crate::abootimg::AbootimgF;
use crate::ar::UnixArF;
use crate::bzip::BzipF;
use crate::der_cert::DerCertF;
use crate::dtb::DtbF;
use crate::erofs::ErofsF;
use crate::ext4::Ext4F;
use crate::fbpack::FbpackF;
use crate::file_ref::FileRef;
use crate::hmfs::HmfsF;
use crate::linux_zimg::LinuzZImgF;
use crate::lzma::LzmaF;
use crate::mapped_file::MappedFile;
use crate::mbr::MbrF;
use crate::md1img::Md1imgF;
use crate::mtk_dbginfo::MtkDbgF;
use crate::ohos::OhosF;
use crate::pbzx::PbzxF;
use crate::pem_cert::PemCertF;
use crate::protbuf_raw::PBufF;
use crate::qcow2::Qcow2F;
use crate::rar::RarFile;
use crate::sniff_allwinner::AllwinnerA10F;
use crate::sniff_dtbo::DtboF;
use crate::sniff_esp32::Esp32F;
use crate::sniff_f2fs::F2fsF;
use crate::sniff_mtk_hblr::MtkHblrF;
use crate::sniff_shannon::ShannonF;
use crate::tar::TarFile;
use crate::update_app::UpdateAppF;
use crate::upx::UpxF;
use crate::xar::XarF;
use crate::xz::XzF;
use crate::zip::ZipFile;
use crate::{
    android_sparse::SparseF, gen_item::GenItem, lz4::Lz4F, lzfse::LzfseF, lzo::LzoF, qcom_ptbl::Ptbl,
    sniff_vbmeta::VbmetaF, tlv::TlvF, vendor_boot::VendorBoot,
};
use crate::{chomeos_ota::ChromeosOTAF, cpio::CpioFile, gzip::GzipF};
use crate::{liblp::LibLPf, mx140::Mx140F};

pub trait GenFS {
    fn try_open(f: &FileRef) -> anyhow::Result<Option<Self>>
    where
        Self: Sized,
    {
        if Self::sniff(f.mmap)? {
            return Ok(Some(Self::try_open_internal(f)?));
        }
        Ok(None)
    }

    fn try_open_internal(_f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized;

    fn sniff(_f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        Ok(true)
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>>;

    fn name(&self) -> &str;

    fn single_output(&self) -> bool {
        false
    }

    fn sniff_only(&self) -> bool {
        false
    }
}

pub fn try_open_mem(f: MappedFile, format: Option<&String>) -> Option<Box<dyn GenFS>> {
    let fref = f.get_ref();

    macro_rules! try_format {
        ($typ: ident, $def: expr, $name: expr) => {
            if format.map(|c| c == $typ::FORMAT_NAME).unwrap_or($def) {
                match $typ::try_open(&fref) {
                    Ok(Some(f)) => return Some(Box::new(f)),
                    Ok(None) => {}
                    Err(e) => {
                        println!("{} - {:?}", $name, e);
                    }
                }
            }
        };
    }

    try_format!(ZipFile, true, "zip");
    try_format!(GzipF, true, "gzip");
    try_format!(BzipF, true, "bzip");
    try_format!(UpxF, true, "upx");
    try_format!(PBufF, true, "protobuf");
    try_format!(LinuzZImgF, true, "linux_zimg");
    try_format!(DtboF, true, "dtbo");
    try_format!(DtbF, true, "dtb");
    try_format!(ShannonF, true, "shannon");
    try_format!(Ext4F, false, "ext4");
    try_format!(PemCertF, true, "pem_cert");
    try_format!(DerCertF, true, "der_cert");
    try_format!(Md1imgF, true, "mtk_md1img");
    try_format!(HmfsF, true, "HmfsF");
    try_format!(Qcow2F, true, "qcow2f");
    try_format!(AbootimgF, true, "abootimg");
    try_format!(OhosF, true, "ohos");
    try_format!(ErofsF, true, "erofs");
    try_format!(CpioFile, true, "cpio");
    try_format!(TarFile, true, "tar");
    try_format!(UnixArF, true, "ar");
    try_format!(LibLPf, true, "lpf");
    try_format!(SparseF, true, "sparse");
    try_format!(Lz4F, true, "lz4");
    try_format!(LzfseF, true, "lzfse");
    try_format!(LzoF, true, "lzo");
    try_format!(VendorBoot, true, "vendorboot");
    try_format!(Ptbl, true, "lpf");
    try_format!(TlvF, false, "tlvf");
    try_format!(Mx140F, false, "mx140");
    try_format!(ChromeosOTAF, true, "chromeota");
    try_format!(MbrF, true, "MBR");
    try_format!(FbpackF, true, "fbpack");
    try_format!(F2fsF, true, "f2fs");
    try_format!(MtkHblrF, true, "HBLR");
    try_format!(LzmaF, true, "lzma");
    try_format!(MtkDbgF, true, "mtk_dbg");
    try_format!(Esp32F, true, "esp32_fw");
    try_format!(UpdateAppF, true, "update_app");
    try_format!(XzF, true, "xz");
    try_format!(RarFile, false, "rar");
    try_format!(AllwinnerA10F, true, "allwinner_a10");
    try_format!(VbmetaF, true, "vbmeta");
    try_format!(PbzxF, true, "pbzx");
    try_format!(XarF, true, "xar");

    None
}

pub fn try_open(p: &Path, format: Option<&String>) -> Option<Box<dyn GenFS>> {
    if let Ok(meta) = std::fs::metadata(p)
        && meta.size() > 1024 * 1024 * 1024 * 2
    {
        warn!("This file is > 2G, hope you have ram for that...");
    }

    if !std::fs::exists(p).unwrap_or(false) {
        warn!("File '{}' doesn't exist", p.display());
        return None;
    }

    let f = MappedFile::open(p);

    try_open_mem(f, format)
}
