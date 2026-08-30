pub mod gen_item;
pub mod generic_fs;

pub mod abootimg;
pub mod android_sparse;
pub mod ar;
pub mod bzip;
pub mod chomeos_ota;
pub mod cpio;
pub mod der_cert;
pub mod dtb;
pub mod erofs;
pub mod ext4;
pub mod fbpack;
pub mod file_ref;
mod generic_fs_props;
pub mod gzip;
pub mod hmfs;
pub mod liblp;
pub mod linux_zimg;
pub mod lz4;
pub mod lzfse;
pub mod lzo;
pub mod lzma;
pub mod mapped_file;
pub mod mbr;
pub mod md1img;
pub mod mtk_dbginfo;
pub mod mx140;
pub mod ohos;
pub mod pbzx;
pub mod pem_cert;
pub mod protbuf_raw;
pub mod qcom_ptbl;
pub mod qcow2;
pub mod rar;
pub mod sniff_allwinner;
pub mod sniff_dtbo;
pub mod sniff_esp32;
pub mod sniff_f2fs;
pub mod sniff_mtk_hblr;
pub mod sniff_shannon;
pub mod sniff_vbmeta;
pub mod tar;
pub mod tlv;
pub mod update_app;
pub mod upx;
pub mod vendor_boot;
pub mod wip_garmin;
pub mod xar;
pub mod xz;
pub mod zip;
pub mod sniff_yaa;
// hsp: no libs
// UBI-flash?
// EXE resources
// yaffs2

// tar uses too much memory, write our own lib
