use crate::fuse::inode_state::InodeState;
use crate::fuse::{VERBOSE_LOG, VERBOSE_LOG_READ};
use crate::generic_fs::GenFS;
use fuse::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEmpty, ReplyEntry,
    ReplyOpen, ReplyXattr, Request,
};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::io::{Seek, SeekFrom};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{Duration, Instant};
use time::Timespec;
use tracing::{info, warn};

pub struct AutoUnpackFs {
    pub base_path: String,
    pub cont: Box<dyn GenFS>,
    pub id: u64,
    pub inode_look: BTreeMap<(u64, String), InodeState>,
    pub anti_hang: Instant,
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
            reply.error(0);
        }
    }

    fn getattr(&mut self, _req: &Request, _ino: u64, reply: ReplyAttr) {
        if VERBOSE_LOG {
            println!("[{}] getattr _ino={_ino}", self.base_path);
        }

        if _ino == 1 || _ino == 2 {
            let t = Timespec::new(1, 0);
            reply.attr(
                &t,
                &FileAttr {
                    ino: _ino,
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
        } else if let Some((_, i)) = self.inode_look.iter().find(|(_a, b)| b.id == _ino) {
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
            reply.error(0);
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
        _ino: u64,
        _fh: u64,
        _offset: i64,
        mut reply: ReplyDirectory,
    ) {
        if VERBOSE_LOG {
            println!("[{}] readdir ino={_ino} off={_offset}", self.base_path);
        }

        self.anti_hang = Instant::now();

        if _ino == 1 && _offset == 0 {
            reply.add(1, 2, FileType::RegularFile, ".");
            reply.add(2, 3, FileType::RegularFile, "..");

            if !self.inode_look.is_empty() {
                for ((_pid, _ino), i) in &self.inode_look {
                    reply.add(i.id, (i.id + 1) as _, i.kind, i.name.clone());
                }
            } else {
                while let Ok(Some(f)) = self.cont.next_itm() {
                    let name = f.name();
                    let s = f.size();

                    if name.contains("/") {
                        warn!("Currently mount can't do dirs");
                        continue;
                    }

                    info!("Adding {name}");

                    reply.add(
                        self.id,
                        (self.id + 1) as _,
                        FileType::RegularFile,
                        name.clone(),
                    );
                    self.inode_look.insert(
                        (_ino, name.clone()),
                        InodeState {
                            kind: FileType::RegularFile,
                            size: s,
                            id: self.id,
                            name: name.clone(),
                            f: Some(f),
                        },
                    );
                    self.id += 1;

                    reply.add(
                        self.id,
                        (self.id + 1) as _,
                        FileType::Directory,
                        format!("_{name}"),
                    );
                    self.inode_look.insert(
                        (_ino, format!("_{name}").clone()),
                        InodeState {
                            kind: FileType::Directory,
                            size: s,
                            id: self.id,
                            name: format!("_{name}"),
                            f: None,
                        },
                    );
                    self.id += 1;

                    crate::fuse::fuse_mnt::wait_and_mount(
                        PathBuf::from_str(&format!("{}/_{name}", self.base_path)).unwrap(),
                        PathBuf::from_str(&format!("{}/{name}", self.base_path)).unwrap(),
                    );
                }
            }

            reply.ok();
        } else {
            // let e = self.inode_look.iter().find(|(_k, v)| v.id == _ino).unwrap();
            // println!("{:?}", e.0);

            if _offset == 0 {
                println!("{_ino} is a nest?");
                reply.add(_ino, (_ino + 1) as _, FileType::RegularFile, ".");
                // self.id += 1;
                reply.add(1, 10, FileType::RegularFile, "..");
                // self.id += 1;
            }

            reply.ok();
        }
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

    fn open(&mut self, _req: &Request, _ino: u64, _flags: u32, reply: ReplyOpen) {
        println!("open ino={_ino}, {:?}", _req);
        let ((_parent, _path), _i) = self.inode_look.iter().find(|(_a, i)| i.id == _ino).unwrap();
        println!("{:?}", _path);

        self.id += 1;

        reply.opened(self.id, _flags);
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
        _ino: u64,
        _fh: u64,
        offset: i64,
        _size: u32,
        reply: ReplyData,
    ) {
        if VERBOSE_LOG_READ {
            info!("read ino={_ino}, {:?}, offset={offset}", _req);
        }
        if Instant::now().duration_since(self.anti_hang) > Duration::from_secs(30) {
            println!("ANTI HANG");
            reply.error(0);
            return;
        }

        let ((_parent, _path), i) = self
            .inode_look
            .iter_mut()
            .find(|(_a, i)| i.id == _ino)
            .unwrap();

        let f = i.f.as_mut().unwrap();

        f.seek(SeekFrom::Start(offset as _)).unwrap();

        let mut buf = vec![0; _size as usize];

        let sz = f.read(&mut buf);
        if sz == 0 {
            reply.error(0);
        } else {
            let buf = &buf[..sz as usize];
            reply.data(buf);
        }
    }
}
