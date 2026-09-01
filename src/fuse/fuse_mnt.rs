use crate::fuse::RECURSIVE_MNT;
use crate::fuse::auto_unpack_fs::AutoUnpackFs;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::str::FromStr;
use std::thread;
use std::time::{Duration, Instant};
use tracing::info;

pub fn fuse_mount(pth: String, src: String) {
    let z = genericfs::generic_fs::try_open(std::path::Path::new(&src), None)
        .expect("Can't mount this");

    let a = AutoUnpackFs {
        base_path: std::path::absolute(std::path::Path::new(&pth))
            .unwrap()
            .to_string_lossy()
            .to_string(),
        cont: z,
        id: 3,
        inode_look: Default::default(),
        anti_hang: Instant::now(),
    };

    info!("mounting {} on {}", a.cont.name(), pth);

    fuse::mount(
        a,
        &PathBuf::from_str(&pth).unwrap(),
        &[
            OsStr::new("-o"),
            OsStr::new("allow_other"),
            // OsStr::new("-o"), OsStr::new("auto_unmount"),
            OsStr::new("-o"),
            OsStr::new("rootmode=777"),
            OsStr::new("-o"),
            OsStr::new("kernel_cache"),
            OsStr::new("-o"),
            OsStr::new("max_read=0"),
        ],
    )
    .unwrap()
}

pub fn wait_and_mount(pp: PathBuf, src: PathBuf) {
    if !RECURSIVE_MNT {
        return;
    }
    std::thread::spawn(move || {
        while !pp.exists() {
            println!("D {}, pp={}", src.display(), pp.display());
            println!("Waiting");
        }

        thread::sleep(Duration::from_secs(
            (1 + pp.file_name().unwrap().len() / 3) as u64,
        ));

        loop {
            if let Some(gc) = genericfs::generic_fs::try_open(&src, None) {
                let a = AutoUnpackFs {
                    base_path: pp.to_string_lossy().to_string(),
                    inode_look: Default::default(),
                    id: 3,
                    cont: gc,
                    anti_hang: Instant::now(),
                };

                if fuse::mount(
                    a,
                    &pp,
                    &[
                        OsStr::new("-o"),
                        OsStr::new("allow_other"),
                        // OsStr::new("-o"), OsStr::new("auto_unmount"),
                        OsStr::new("-o"),
                        OsStr::new("rootmode=777"),
                        OsStr::new("-o"),
                        OsStr::new("kernel_cache"),
                        OsStr::new("-o"),
                        OsStr::new("max_read=0"),
                        // OsStr::new("-o"), OsStr::new("uid=1000"),
                        // OsStr::new("-o"), OsStr::new("gid=1000"),
                    ],
                )
                .is_ok()
                {
                    break;
                }
                // println!("Trying again");
            }
        }
    });
}
