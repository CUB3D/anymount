#![no_main]

use libfuzzer_sys::fuzz_target;
use memfile::{MemFile, CreateOptions};
use std::io::Write;

fuzz_target!(|data: &[u8]| {
    let mut file = MemFile::create("foo", CreateOptions::new()).unwrap();
    file.write_all(data).unwrap();
    let f = file.into_file();
    let mem = genericfs::mapped_file::MappedFile::from_file(f);

    genericfs::generic_fs::try_open_mem(mem, None);
});
