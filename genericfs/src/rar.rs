use rar_stream::{FileMedia, InnerFile, ParseOptions, RarFilesPackage};
pub use std::io::Read;
use std::sync::Arc;
use tracing::info;

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};

#[derive(Debug)]
pub struct RarFile {
    pub files: Vec<InnerFile>,
    pub idx: usize,
}

impl GenFSProps for RarFile {
    const FORMAT_NAME: &'static str = "rar";
    const LOW_CONFIDENCE_SNIFF: bool = true;
}

impl GenFS for RarFile {
    fn try_open_internal(_f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let media: Arc<dyn FileMedia> = Arc::new(rar_stream::LocalFileMedia::new(
            "/home/cub3d/tmp/8C800.rar",
        )?);
        let pkg = RarFilesPackage::new(vec![media]);

        let res = tokio::runtime::Builder::new_current_thread()
            .build()?
            .block_on(async move {
                info!("{:?}", pkg.get_archive_info().await);
                pkg.parse(ParseOptions::default()).await
            })?;

        println!("{}", res.len());
        for f in &res {
            info!("{:?}", f.name);
        }

        Ok(Self { files: res, idx: 0 })
    }

    // fn sniff(f: &Mmap) -> anyhow::Result<bool>
    // where
    //     Self: Sized,
    // {
    //     let hdr = f.get(..4);
    //
    //     if hdr != Some(&[0x50, 0x4b, 0x03, 0x04])
    //         && hdr != Some(&[0x50, 0x4b, 0x05, 0x06])
    //         && hdr != Some(&[0x50, 0x4b, 0x07, 0x08])
    //     {
    //         return Ok(false);
    //     }
    //
    //     Ok(true)
    // }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        if let Some(f) = self.files.get(self.idx) {
            self.idx += 1;
            let d = tokio::runtime::Builder::new_current_thread()
                .build()?
                .block_on(async move { f.read_to_end().await })?;
            return Ok(Some(Box::new(BufGenItm::new(f.name.clone(), d))));
        }

        Ok(None)
    }

    fn name(&self) -> &str {
        "RAR"
    }
}
