use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{gen_item::GenItem, generic_fs::GenFS};
use memmap2::Mmap;
use std::io::{ErrorKind, Seek, SeekFrom};
use std::{
    fs::File,
    io::Cursor,
    sync::{Arc, RwLock},
};

pub struct SparseF {
    pub f: Arc<RwLock<android_sparse::read::Reader<File>>>,
    pub idx: usize,
}

impl GenFSProps for SparseF {
    const FORMAT_NAME: &'static str = "sparse";
}

impl GenFS for SparseF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let f = android_sparse::read::Reader::new(f.owned_file())?;

        Ok(SparseF {
            f: Arc::new(RwLock::new(f)),
            idx: 0,
        })
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        if android_sparse::read::Reader::new(&f[..]).is_ok() {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        if self.idx == 1 {
            return Ok(None);
        }

        // let mut out = Cursor::new(Vec::new());
        // let mut d: Decoder<Cursor<Vec<u8>>> = android_sparse::Decoder::new(out)?;

        self.idx += 1;

        Ok(Some(Box::new(SparseFile {
            f: Arc::clone(&self.f),
            buf: Vec::new(),
            // d,
            // buf: out
        })))

        // while let Some(Ok(b)) = self.f.next() {
        //     d.write_block(&b)?;
        // }
        // d.close()?;

        // self.idx += 1;

        // let gen_itm = Box::new(BufGenItm {
        //     name: "_dec".to_string(),
        //     data: out.into_inner(),
        //     pos: 0,
        // });
        // Ok(Some(gen_itm))
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }

    fn single_output(&self) -> bool {
        true
    }
}

struct SparseFile {
    pub f: Arc<RwLock<android_sparse::read::Reader<File>>>,
    // d: Decoder<Cursor<Vec<u8>>>,
    buf: Vec<u8>,
}

impl Seek for SparseFile {
    fn seek(&mut self, _pos: SeekFrom) -> std::io::Result<u64> {
        Err(std::io::Error::new(ErrorKind::NotSeekable, ""))
    }
}

impl GenItem for SparseFile {
    fn name(&self) -> String {
        "_dec".to_string()
    }

    fn size(&self) -> u64 {
        u64::MAX
    }

    fn read(&mut self, buf: &mut [u8]) -> u64 {
        if self.buf.len() < buf.len() {
            let mut out = Cursor::new(Vec::new());
            let mut d = android_sparse::Decoder::new(&mut out).unwrap();

            let mut idx = 0;
            let mut f = self.f.write().unwrap();
            while let Some(Ok(b)) = f.next() {
                d.write_block(&b).unwrap();
                idx += 1;
                if idx > 10 {
                    break;
                }
            }
            d.close().unwrap();

            self.buf.extend_from_slice(out.into_inner().as_slice());
        }

        let tmp = &self.buf[..buf.len().min(self.buf.len())];

        if tmp.is_empty() {
            return 0;
        }

        buf[..tmp.len()].copy_from_slice(tmp);
        let sz = tmp.len() as u64;

        self.buf.drain(0..sz as usize);

        sz
    }
}
