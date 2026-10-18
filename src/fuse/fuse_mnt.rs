use crate::fuse::RECURSIVE_MNT;
use crate::fuse::auto_unpack_fs::AutoUnpackFs;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::str::FromStr;
use std::thread;
use tracing::info;

pub fn fuse_mount(path_to_mount_to: String, file_to_mount: String) {
    let z = genericfs::generic_fs::try_open(std::path::Path::new(&file_to_mount), None)
        .expect("Can't mount this");

    let mut a = AutoUnpackFs {
        base_path: std::path::absolute(std::path::Path::new(&path_to_mount_to))
            .expect("Can't get absolute path")
            .to_string_lossy()
            .to_string(),
        container: z,
        id: 3,
        inode_look: Default::default(),
    };
    a.build_dir_tree();

    info!("mounting {} on {}", a.container.name(), path_to_mount_to);

    fuse::mount(
        a,
        &PathBuf::from_str(&path_to_mount_to).unwrap(),
        &[
            OsStr::new("-o"),
            OsStr::new("allow_other"),
            OsStr::new("-o"), OsStr::new("auto_unmount"),
            OsStr::new("-o"),
            OsStr::new("rootmode=777"),
            OsStr::new("-o"),
            OsStr::new("kernel_cache"),
            OsStr::new("-o"),
            OsStr::new("max_read=0"),
        ],
    )
    .expect("Failed to fuse mount")
}

pub fn wait_and_mount(pp: PathBuf, src: PathBuf) {
    if !RECURSIVE_MNT {
        return;
    }
    thread::spawn(move || {
        while !pp.exists() {
            println!("D {}, pp={}", src.display(), pp.display());
            println!("Waiting");
        }

        let mut tries = 0;

        while tries < 100 {
            if let Some(gc) = genericfs::generic_fs::try_open(&src, None) {
                let mut a = AutoUnpackFs {
                    base_path: pp.to_string_lossy().to_string(),
                    inode_look: Default::default(),
                    id: 3,
                    container: gc,
                };
                a.build_dir_tree();

                if fuse::mount(
                    a,
                    &pp,
                    &[
                        OsStr::new("-o"),
                        OsStr::new("allow_other"),
                        OsStr::new("-o"), OsStr::new("auto_unmount"),
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
            }
            tries += 1;
        }
    });
}
