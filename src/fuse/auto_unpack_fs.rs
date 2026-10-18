use crate::fuse::inode_state::InodeState;
use crate::fuse::{VERBOSE_LOG, VERBOSE_LOG_READ};
use fuse::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEmpty, ReplyEntry,
    ReplyOpen, ReplyXattr, Request,
};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::io::{Seek, SeekFrom};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use time::Timespec;
use tracing::{info, warn};

pub struct AutoUnpackFs {
    pub base_path: String,
    pub container: Box<dyn genericfs::generic_fs::GenFS>,
    pub id: u64,
    pub inode_look: BTreeMap<(u64, String), InodeState>,
    pub anti_hang: Instant,
}

impl AutoUnpackFs {
    fn lookup_or_create_entry(&mut self, parent: u64, name: &str) -> u64 {
        if let Some(i) = self.inode_look.get(&(parent, name.to_string())) {
            return i.id;
        }

        let id = self.id;
        self.id += 1;
        self.inode_look.insert(
            (parent, name.to_string()),
            InodeState {
                kind: FileType::Directory,
                size: 0,
                id,
                name: name.to_string(),
                f: None,
            },
        );
        id
    }

    pub fn build_dir_tree(&mut self) {
        loop {
            let f = match self.container.next_itm() {
                Ok(Some(f)) => f,
                Ok(None) => break,
                Err(e) => {
                    warn!("Failed to read next item: {e}");
                    break;
                }
            };

            let raw_name = f.name();
            let parts = raw_name
                .split('/')
                .collect::<Vec<_>>();


            if let Some((name, dirs)) = parts.split_last() {
                let mut parent = 1;

                // Create the child dirs
                for d in dirs {
                    parent = self.lookup_or_create_entry(parent, d);
                }

                // Create the file
                let size = f.size();
                self.inode_look.insert(
                    (parent, name.to_string()),
                    InodeState {
                        kind: FileType::RegularFile,
                        size,
                        id: self.id,
                        name: name.to_string(),
                        f: Some(f),
                    },
                );
                self.id += 1;

                self.lookup_or_create_entry(parent, &format!("_{name}"));

                let parent_path = dirs.join("/");
                let (dst, src) = if parent_path.is_empty() {
                    (format!("{}/_{}", self.base_path, name), format!("{}/{}", self.base_path, name))
                } else {
                    (format!("{}/{}/_{}", self.base_path, parent_path, name), format!("{}/{}/{}", self.base_path, parent_path, name))
                };

                crate::fuse::fuse_mnt::wait_and_mount(PathBuf::from(dst), PathBuf::from(src));
            }
        }
    }
}

impl Filesystem for AutoUnpackFs {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        if VERBOSE_LOG {
            println!("[{}] look {:?}", self.base_path, name);
        }
        // println!("look1 {:?}", self.inode_look.get(&(parent, name.to_str().unwrap().to_string())));

        if let Some(i) = self
            .inode_look
            .get(&(parent, name.to_str().unwrap().to_string()))
        {
            // if VERBOSE_LOG {
            //     println!("[{}] look got {:?}", self.base_path, i);
            // }
            let t = Timespec::new(1, 0);
            reply.entry(
                &t,
                &FileAttr {
                    ino: i.id,
                    size: i.size,
                    blocks: 1,
                    atime: t,
                    mtime: t,
                    ctime: t,
                    crtime: t,
                    kind: i.kind,
                    perm: 0o777,
                    nlink: 2,
                    uid: 1000,
                    gid: 1000,
                    rdev: 0,
                    flags: 0,
                },
                0,
            );
        } else {
            if VERBOSE_LOG {
                println!("look Not here");
            }
            reply.error(libc::ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        if VERBOSE_LOG {
            println!("[{}] getattr _ino={ino}", self.base_path);
        }

        if ino == 1 || ino == 2 {
            let t = Timespec::new(1, 0);
            reply.attr(
                &t,
                &FileAttr {
                    ino,
                    size: 0x1000,
                    blocks: 1,
                    atime: t,
                    mtime: t,
                    ctime: t,
                    crtime: t,
                    kind: FileType::Directory,
                    perm: 0o777,
                    nlink: 2,
                    uid: 1000,
                    gid: 1000,
                    rdev: 0,
                    flags: 0,
                },
            );
        } else if let Some((_, i)) = self.inode_look.iter().find(|(_a, b)| b.id == ino) {
            // if VERBOSE_LOG {
            //     println!("[{}] ga got {:?}", self.base_path, i);
            // }
            let t = Timespec::new(1, 0);
            reply.attr(
                &t,
                &FileAttr {
                    ino: i.id,
                    size: i.size,
                    blocks: 0,
                    atime: t,
                    mtime: t,
                    ctime: t,
                    crtime: t,
                    kind: i.kind,
                    perm: 0o777,
                    nlink: 2,
                    uid: 1000,
                    gid: 1000,
                    rdev: 0,
                    flags: 0,
                },
            );
        } else {
            reply.error(libc::ENOENT);
        }
    }
    fn opendir(&mut self, _req: &Request, _ino: u64, _flags: u32, reply: ReplyOpen) {
        if VERBOSE_LOG {
            println!("[{}] opendir ino={_ino}", self.base_path);
        }
        reply.opened(0, 0);
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        if VERBOSE_LOG {
            println!("[{}] readdir ino={ino} off={offset}", self.base_path);
        }

        self.anti_hang = Instant::now();

        let parent = if ino == 1 {
            1
        } else {
            self.inode_look
                .iter()
                .find(|((_, _), i)| i.id == ino)
                .map(|((p, _), _)| *p)
                .unwrap_or(1)
        };

        let mut entries = Vec::new();
        entries.push((ino, FileType::Directory, ".".to_string()));
        entries.push((parent, FileType::Directory, "..".to_string()));

        for ((p, _), i) in &self.inode_look {
            if *p == ino {
                entries.push((i.id, i.kind, i.name.clone()));
            }
        }

        let offset = offset.max(0) as usize;
        for (idx, (ino, kind, name)) in entries.iter().enumerate().skip(offset) {
            if reply.add(*ino, (idx + 1) as i64, *kind, name) {
                break;
            }
        }

        reply.ok();
    }

    fn listxattr(&mut self, _req: &Request, _ino: u64, _size: u32, reply: ReplyXattr) {
        println!("listx ino={_ino} {:?}", _req);
        reply.error(38);
    }

    fn getxattr(
        &mut self,
        _req: &Request,
        _ino: u64,
        _name: &OsStr,
        _size: u32,
        reply: ReplyXattr,
    ) {
        println!("listx ino={_ino} {:?}", _req);
        reply.error(38);
    }

    fn open(&mut self, _req: &Request, ino: u64, flags: u32, reply: ReplyOpen) {
        println!("open ino={ino}, {:?}", _req);
        let ((_parent, _path), _i) = self.inode_look.iter().find(|(_a, i)| i.id == ino).unwrap();
        println!("{:?}", _path);

        self.id += 1;

        reply.opened(self.id, flags);
    }

    fn release(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _flags: u32,
        _lock_owner: u64,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        reply.ok()
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        reply: ReplyData,
    ) {
        if VERBOSE_LOG_READ {
            info!("read ino={ino}, {:?}, offset={offset}", _req);
        }
        if Instant::now().duration_since(self.anti_hang) > Duration::from_secs(30) {
            println!("ANTI HANG");
            reply.error(0);
            return;
        }

        let ((_parent, _path), i) = self
            .inode_look
            .iter_mut()
            .find(|(_a, i)| i.id == ino)
            .unwrap();

        let f = i.f.as_mut().unwrap();

        f.seek(SeekFrom::Start(offset as _)).unwrap();

        let mut buf = vec![0; size as usize];

        let sz = f.read(&mut buf);
        if sz == 0 {
            reply.error(libc::EOF);
        } else {
            let buf = &buf[..sz as usize];
            reply.data(buf);
        }
    }
}
