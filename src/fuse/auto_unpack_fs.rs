use crate::fuse::inode_state::InodeState;
use crate::fuse::{VERBOSE_LOG, VERBOSE_LOG_READ};
use fuser::{
    Errno, FileAttr, FileHandle, FileType, Filesystem, FopenFlags, Generation, INodeNo, LockOwner,
    OpenFlags, ReplyAttr, ReplyData, ReplyDirectory, ReplyEmpty, ReplyEntry, ReplyOpen, ReplyXattr,
    Request,
};
use genericfs::generic_fs::GenFS;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::io::{Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use tracing::{info, warn};

pub struct FsState {
    pub id: u64,
    pub inode_look: BTreeMap<(u64, String), InodeState>,
}

impl FsState {
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
}

pub struct AutoUnpackFs {
    pub base_path: String,
    pub state: Mutex<FsState>,
}

impl AutoUnpackFs {
    pub fn new(base_path: String, genfs: Box<dyn GenFS>) -> Self {
        let mut s = Self {
            base_path,
            state: Mutex::new(FsState {
                id: 3,
                inode_look: Default::default(),
            }),
        };
        s.build_dir_tree(genfs);
        s
    }

    pub fn build_dir_tree(&mut self, mut container: Box<dyn genericfs::generic_fs::GenFS>) {
        let mut state = self.state.lock().unwrap();

        loop {
            let f = match container.next_itm() {
                Ok(Some(f)) => f,
                Ok(None) => break,
                Err(e) => {
                    warn!("Failed to read next item: {e}");
                    break;
                }
            };

            let raw_name = f.name();
            let parts = raw_name.split('/').collect::<Vec<_>>();

            if let Some((name, dirs)) = parts.split_last() {
                let mut parent = 1;

                // Create the child dirs
                for d in dirs {
                    parent = state.lookup_or_create_entry(parent, d);
                }

                // Create the file
                let size = f.size();
                let id = state.id;
                state.inode_look.insert(
                    (parent, name.to_string()),
                    InodeState {
                        kind: FileType::RegularFile,
                        size,
                        id,
                        name: name.to_string(),
                        f: Some(f),
                    },
                );
                state.id += 1;

                state.lookup_or_create_entry(parent, &format!("_{name}"));

                let parent_path = dirs.join("/");
                let (dst, src) = if parent_path.is_empty() {
                    (
                        format!("{}/_{}", self.base_path, name),
                        format!("{}/{}", self.base_path, name),
                    )
                } else {
                    (
                        format!("{}/{}/_{}", self.base_path, parent_path, name),
                        format!("{}/{}/{}", self.base_path, parent_path, name),
                    )
                };

                crate::fuse::fuse_mnt::wait_and_mount(PathBuf::from(dst), PathBuf::from(src));
            }
        }
    }
}

impl Filesystem for AutoUnpackFs {
    fn lookup(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
        if VERBOSE_LOG {
            println!("[{}] look {:?}", self.base_path, name);
        }

        let state = self.state.lock().unwrap();
        if let Some(i) = state
            .inode_look
            .get(&(parent.0, name.to_str().unwrap().to_string()))
        {
            let t = SystemTime::now();
            reply.entry(
                &Duration::from_secs(1),
                &FileAttr {
                    ino: INodeNo(i.id),
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
                    blksize: 4096,
                    flags: 0,
                },
                Generation(0),
            );
        } else {
            if VERBOSE_LOG {
                println!("look Not here");
            }
            reply.error(Errno::ENOENT);
        }
    }

    fn getattr(&self, _req: &Request, ino: INodeNo, _fh: Option<FileHandle>, reply: ReplyAttr) {
        if VERBOSE_LOG {
            println!("[{}] getattr _ino={ino}", self.base_path);
        }

        let state = self.state.lock().unwrap();
        if ino.0 == 1 || ino.0 == 2 {
            let t = SystemTime::now();
            reply.attr(
                &Duration::from_secs(1),
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
                    blksize: 4096,
                    flags: 0,
                },
            );
        } else if let Some((_, i)) = state.inode_look.iter().find(|(_a, b)| b.id == ino.0) {
            let t = SystemTime::now();
            reply.attr(
                &Duration::from_secs(1),
                &FileAttr {
                    ino: INodeNo(i.id),
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
                    blksize: 4096,
                    flags: 0,
                },
            );
        } else {
            reply.error(Errno::ENOENT);
        }
    }

    fn opendir(&self, _req: &Request, _ino: INodeNo, _flags: OpenFlags, reply: ReplyOpen) {
        if VERBOSE_LOG {
            println!("[{}] opendir ino={_ino}", self.base_path);
        }
        reply.opened(FileHandle(0), FopenFlags::empty());
    }

    fn readdir(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        offset: u64,
        mut reply: ReplyDirectory,
    ) {
        if VERBOSE_LOG {
            println!("[{}] readdir ino={ino} off={offset}", self.base_path);
        }

        let state = self.state.lock().unwrap();
        let parent = if ino.0 == 1 {
            1
        } else {
            state
                .inode_look
                .iter()
                .find(|((_, _), i)| i.id == ino.0)
                .map(|((p, _), _)| *p)
                .unwrap_or(1)
        };

        let mut entries = Vec::new();
        entries.push((ino.0, FileType::Directory, ".".to_string()));
        entries.push((parent, FileType::Directory, "..".to_string()));

        for ((p, _), i) in &state.inode_look {
            if *p == ino.0 {
                entries.push((i.id, i.kind, i.name.clone()));
            }
        }

        for (idx, (ino, kind, name)) in entries.iter().enumerate().skip(offset as usize) {
            if reply.add(INodeNo(*ino), (idx + 1) as u64, *kind, name) {
                break;
            }
        }

        reply.ok();
    }

    fn listxattr(&self, _req: &Request, _ino: INodeNo, _size: u32, reply: ReplyXattr) {
        println!("listx ino={_ino} {:?}", _req);
        reply.error(Errno::ENOSYS);
    }

    fn getxattr(
        &self,
        _req: &Request,
        _ino: INodeNo,
        _name: &OsStr,
        _size: u32,
        reply: ReplyXattr,
    ) {
        println!("listx ino={_ino} {:?}", _req);
        reply.error(Errno::ENOSYS);
    }

    fn open(&self, _req: &Request, ino: INodeNo, _flags: OpenFlags, reply: ReplyOpen) {
        if VERBOSE_LOG {
            println!("open ino={ino}, {:?}", _req);
        }

        let mut state = self.state.lock().unwrap();
        let path = state
            .inode_look
            .iter()
            .find(|(_a, i)| i.id == ino.0)
            .map(|((_parent, path), _i)| path.clone());
        println!("{:?}", path);

        state.id += 1;
        let fh = state.id;
        reply.opened(FileHandle(fh), FopenFlags::empty());
    }

    fn release(
        &self,
        _req: &Request,
        _ino: INodeNo,
        _fh: FileHandle,
        _flags: OpenFlags,
        _lock_owner: Option<LockOwner>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        reply.ok()
    }

    fn read(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        offset: u64,
        size: u32,
        _flags: OpenFlags,
        _lock_owner: Option<LockOwner>,
        reply: ReplyData,
    ) {
        if VERBOSE_LOG_READ {
            info!("read ino={ino}, {:?}, offset={offset}", _req);
        }

        let data = {
            let mut state = self.state.lock().unwrap();
            let ((_parent, _path), i) = state
                .inode_look
                .iter_mut()
                .find(|(_a, i)| i.id == ino.0)
                .unwrap();

            let f = i.f.as_mut().unwrap();

            f.seek(SeekFrom::Start(offset)).unwrap();

            let mut buf = vec![0; size as usize];

            let sz = f.read(&mut buf);
            buf.truncate(sz as usize);
            buf
        };

        reply.data(&data);
    }
}
