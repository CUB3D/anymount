use std::io::{Seek, SeekFrom};

/// A Generic Item that can be accessed like a file-ish
pub trait GenItem: Seek {
    fn name(&self) -> String;
    fn size(&self) -> u64;
    fn read(&mut self, buf: &mut [u8]) -> u64;

    fn read_to_vec(&mut self) -> Vec<u8> {
        let mut buf = [0u8; 0x1000];
        let mut tmp = Vec::new();
        loop {
            let p = self.read(&mut buf);
            if p == 0 {
                break;
            }
            tmp.extend_from_slice(&buf[..p as usize]);
        }
        tmp
    }
}

impl Seek for BufGenItm {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        match pos {
            SeekFrom::Start(x) => self.pos = x,
            SeekFrom::End(x) => self.pos = (self.size() as i64 + x) as u64,
            SeekFrom::Current(x) => self.pos = (self.pos as i64).wrapping_add(x) as u64,
        }
        Ok(self.pos)
    }
}

#[derive(Clone)]
pub struct BufGenItm {
    pub name: String,
    pub data: Vec<u8>,
    pub pos: u64,
}

impl BufGenItm {
    pub fn new<T: AsRef<str>>(name: T, data: Vec<u8>) -> Self {
        BufGenItm {
            name: name.as_ref().to_string(),
            data,
            pos: 0,
        }
    }

    pub fn new_empty<T: AsRef<str>>(name: T) -> Self {
        Self::new(name, Vec::new())
    }
}

impl GenItem for BufGenItm {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn size(&self) -> u64 {
        self.data.len() as u64
    }

    fn read(&mut self, buf: &mut [u8]) -> u64 {
        let tmp = &self.data[self.pos as usize..];
        let tmp = &tmp[..buf.len().min(tmp.len())];

        if tmp.is_empty() {
            return 0;
        }

        buf[..tmp.len()].copy_from_slice(tmp);
        let sz = tmp.len() as u64;

        self.pos += sz;

        sz
    }
}
