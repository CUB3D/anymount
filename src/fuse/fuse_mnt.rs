use crate::fuse::RECURSIVE_MNT;
use crate::fuse::auto_unpack_fs::AutoUnpackFs;
use fuser::{Config, MountOption, SessionACL};
use std::path::PathBuf;
use std::str::FromStr;
use std::thread;
use tracing::info;

pub fn fuse_mount(path_to_mount_to: String, file_to_mount: String) {
    let z = genericfs::generic_fs::try_open(std::path::Path::new(&file_to_mount), None)
        .expect("Can't mount this");

    let base_path = std::path::absolute(std::path::Path::new(&path_to_mount_to))
        .expect("Can't get absolute path")
        .to_string_lossy()
        .to_string();

    info!("mounting {} on {path_to_mount_to}", z.name());

    let a = AutoUnpackFs::new(base_path, z);

    let mut config = Config::default();
    config.mount_options = vec![
        MountOption::AutoUnmount,
        MountOption::CUSTOM("rootmode=777".to_string()),
        MountOption::CUSTOM("max_read=0".to_string()),
    ];
    config.acl = SessionACL::All;

    fuser::mount(a, PathBuf::from_str(&path_to_mount_to).unwrap(), &config)
        .expect("Failed to fuse mount")
}

pub fn wait_and_mount(pp: PathBuf, src: PathBuf) {
    if !RECURSIVE_MNT {
        return;
    }
    thread::spawn(move || {
        // Wait for target dir to exist
        while !pp.exists() {
            // println!("D {}, pp={}", src.display(), pp.display());
            // println!("Waiting");
            thread::sleep(std::time::Duration::from_millis(100));
        }

        let mut tries = 0;

        while tries < 100 {
            if let Some(gc) = genericfs::generic_fs::try_open(&src, None) {
                let a = AutoUnpackFs::new(pp.to_string_lossy().to_string(), gc);

                let mut config = Config::default();
                config.mount_options = vec![
                    MountOption::AutoUnmount,
                    MountOption::CUSTOM("rootmode=777".to_string()),
                    MountOption::CUSTOM("max_read=0".to_string()),
                ];
                config.acl = SessionACL::All;

                if fuser::mount(a, &pp, &config).is_ok() {
                    break;
                }
            }
            tries += 1;
            thread::sleep(std::time::Duration::from_millis(100));
        }
    });
}
