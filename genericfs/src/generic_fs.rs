use crate::generic_fs_props::GenFSProps;
use memmap2::Mmap;
use std::marker::PhantomData;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use tracing::{debug, warn};

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
use crate::img4::Img4F;
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
use crate::rpm::RpmF;
use crate::sniff_7z::SevenZipF;
use crate::sniff_allwinner::AllwinnerA10F;
use crate::sniff_bootldr::BootldrF;
use crate::sniff_dtbh::DtbhF;
use crate::sniff_dtbo::DtboF;
use crate::sniff_esp32::Esp32F;
use crate::sniff_f2fs::F2fsF;
use crate::sniff_fbpt::FbptF;
use crate::sniff_ftab::FtabF;
use crate::sniff_mtk_hblr::MtkHblrF;
use crate::sniff_shannon::ShannonF;
use crate::sniff_uimage::UbootUImgF;
use crate::sniff_yaa::YaaF;
use crate::sniff_zowie::ZowieboxF;
use crate::tar::TarFile;
use crate::update_app::UpdateAppF;
use crate::upx::UpxF;
use crate::xar::XarF;
use crate::xz::XzF;
use crate::zip::ZipFile;
use crate::{
    android_sparse::SparseF, gen_item::GenItem, lz4::Lz4F, lzfse::LzfseF, lzo::LzoF,
    qcom_ptbl::Ptbl, sniff_vbmeta::VbmetaF, tlv::TlvF, vendor_boot::VendorBoot,
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

pub trait GenericFSHelper {
    fn try_open(&self, f: &FileRef) -> anyhow::Result<Option<Box<dyn GenFS>>>;

    fn enabled_by_default(&self) -> bool;

    fn format_name(&self) -> &'static str;
}


/// Helper for working with implementors of GenFS, unlike genfs this can be instantiated without a data source so can be stored in a global instance
struct GenericFSHelperImpl<T>(PhantomData<*mut T>);

impl<T: GenFS + GenFSProps + 'static> GenericFSHelper for GenericFSHelperImpl<T> {
    fn try_open(&self, f: &FileRef) -> anyhow::Result<Option<Box<dyn GenFS>>> {
        let x: Option<T> = T::try_open(f)?;
        let x: Option<Box<dyn GenFS>> = x.map(|x| Box::new(x) as Box<dyn GenFS>);
        Ok(x)
    }
    
    fn enabled_by_default(&self) -> bool {
        !T::LOW_CONFIDENCE_SNIFF
    }
    
    fn format_name(&self) -> &'static str {
        T::FORMAT_NAME
    }
}

impl<T: GenFS> GenericFSHelperImpl<T> {
    const INSTANCE: Self = Self(PhantomData{});
}

pub const FORMATS: &[&dyn GenericFSHelper] = &[
    &GenericFSHelperImpl::<ZipFile>::INSTANCE,
    &GenericFSHelperImpl::<GzipF>::INSTANCE,
    &GenericFSHelperImpl::<BzipF>::INSTANCE,
    &GenericFSHelperImpl::<UpxF>::INSTANCE,
    &GenericFSHelperImpl::<PBufF>::INSTANCE,
    &GenericFSHelperImpl::<LinuzZImgF>::INSTANCE,
    &GenericFSHelperImpl::<DtboF>::INSTANCE,
    &GenericFSHelperImpl::<DtbF>::INSTANCE,
    &GenericFSHelperImpl::<ShannonF>::INSTANCE,
    &GenericFSHelperImpl::<Ext4F>::INSTANCE,
    &GenericFSHelperImpl::<PemCertF>::INSTANCE,
    &GenericFSHelperImpl::<DerCertF>::INSTANCE,
    &GenericFSHelperImpl::<Md1imgF>::INSTANCE,
    &GenericFSHelperImpl::<HmfsF>::INSTANCE,
    &GenericFSHelperImpl::<Qcow2F>::INSTANCE,
    &GenericFSHelperImpl::<AbootimgF>::INSTANCE,
    &GenericFSHelperImpl::<OhosF>::INSTANCE,
    &GenericFSHelperImpl::<ErofsF>::INSTANCE,
    &GenericFSHelperImpl::<CpioFile>::INSTANCE,
    &GenericFSHelperImpl::<TarFile>::INSTANCE,
    &GenericFSHelperImpl::<UnixArF>::INSTANCE,
    &GenericFSHelperImpl::<LibLPf>::INSTANCE,
    &GenericFSHelperImpl::<SparseF>::INSTANCE,
    &GenericFSHelperImpl::<Lz4F>::INSTANCE,
    &GenericFSHelperImpl::<LzfseF>::INSTANCE,
    &GenericFSHelperImpl::<LzoF>::INSTANCE,
    &GenericFSHelperImpl::<VendorBoot>::INSTANCE,
    &GenericFSHelperImpl::<Ptbl>::INSTANCE,
    &GenericFSHelperImpl::<TlvF>::INSTANCE,
    &GenericFSHelperImpl::<Mx140F>::INSTANCE,
    &GenericFSHelperImpl::<ChromeosOTAF>::INSTANCE,
    &GenericFSHelperImpl::<MbrF>::INSTANCE,
    &GenericFSHelperImpl::<FbpackF>::INSTANCE,
    &GenericFSHelperImpl::<F2fsF>::INSTANCE,
    &GenericFSHelperImpl::<MtkHblrF>::INSTANCE,
    &GenericFSHelperImpl::<LzmaF>::INSTANCE,
    &GenericFSHelperImpl::<MtkDbgF>::INSTANCE,
    &GenericFSHelperImpl::<Esp32F>::INSTANCE,
    &GenericFSHelperImpl::<UpdateAppF>::INSTANCE,
    &GenericFSHelperImpl::<XzF>::INSTANCE,
    &GenericFSHelperImpl::<RarFile>::INSTANCE,
    &GenericFSHelperImpl::<RpmF>::INSTANCE,
    &GenericFSHelperImpl::<AllwinnerA10F>::INSTANCE,
    &GenericFSHelperImpl::<VbmetaF>::INSTANCE,
    &GenericFSHelperImpl::<PbzxF>::INSTANCE,
    &GenericFSHelperImpl::<XarF>::INSTANCE,
    &GenericFSHelperImpl::<YaaF>::INSTANCE,
    &GenericFSHelperImpl::<FbptF>::INSTANCE,
    &GenericFSHelperImpl::<DtbhF>::INSTANCE,
    &GenericFSHelperImpl::<Img4F>::INSTANCE,
    &GenericFSHelperImpl::<ZowieboxF>::INSTANCE,
    &GenericFSHelperImpl::<FtabF>::INSTANCE,
    &GenericFSHelperImpl::<UbootUImgF>::INSTANCE,
    &GenericFSHelperImpl::<BootldrF>::INSTANCE,
    &GenericFSHelperImpl::<SevenZipF>::INSTANCE,
];


pub fn try_open_mem(f: MappedFile, format: Option<&String>) -> Option<Box<dyn GenFS>> {
    let fref = f.get_ref();

    for f in FORMATS {
        debug!("Probe fmt {}", f.format_name());
        if format.map(|c| c == f.format_name()).unwrap_or(f.enabled_by_default()) {
                match f.try_open(&fref) {
                    Ok(Some(f)) => return Some(f),
                    Ok(None) => {}
                    Err(e) => {
                        println!("{} - {:?}", f.format_name(), e);
                    }
                }
            }
    }

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
