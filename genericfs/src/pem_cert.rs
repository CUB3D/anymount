//! PEM cert

use crate::file_ref::FileRef;
use crate::generic_fs_props::GenFSProps;
use crate::{
    gen_item::{BufGenItm, GenItem},
    generic_fs::GenFS,
};
use memmap2::Mmap;

pub struct PemCertF {
    pub idx: usize,
    pub o: Vec<BufGenItm>,
}

impl GenFSProps for PemCertF {
    const FORMAT_NAME: &'static str = "pem";
}

impl GenFS for PemCertF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let mut o = Vec::new();

        let mmap = f.mmap;
        let x509 = openssl::x509::X509::from_pem(mmap)?;
        let pkey = x509.public_key()?;
        let mod_ = pkey.rsa()?;
        let mod_ = mod_.n();

        o.push(BufGenItm::new(
            "_modulus.txt",
            format!("Modulus: {}", mod_.to_dec_str()?)
                .as_bytes()
                .to_vec(),
        ));

        Ok(Self { o, idx: 0 })
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        if f.len() as u64 > (i32::MAX) as u64 {
            return Err(anyhow::anyhow!("Openssl has a 2GB file limit"));
        }
        Ok(openssl::x509::X509::from_pem(f).is_ok())
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
