use fuse::FileType;
use genericfs::gen_item::GenItem;

pub struct InodeState {
    pub kind: FileType,
    pub id: u64,
    pub size: u64,
    pub name: String,
    pub f: Option<Box<dyn GenItem>>,
}
