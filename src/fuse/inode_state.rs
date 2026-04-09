use crate::gen_item::GenItem;
use fuse::FileType;

pub struct InodeState {
    pub kind: FileType,
    pub id: u64,
    pub size: u64,
    pub name: String,
    pub f: Option<Box<dyn GenItem>>,
}
