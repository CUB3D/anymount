//! Portable Executable (PE) files

use crate::file_ref::FileRef;
use crate::gen_item::{BufGenItm, GenItem};
use crate::generic_fs::GenFS;
use crate::generic_fs_props::GenFSProps;
use goblin::pe::PE;
use goblin::pe::data_directories::DataDirectory;
use goblin::pe::options::ParseOptions;
use goblin::pe::resource::{ImageResourceDirectory, RT_ACCELERATOR, RT_ANICURSOR, RT_ANIICON, RT_BITMAP, RT_CURSOR, RT_DIALOG, RT_DLGINCLUDE, RT_FONT, RT_FONTDIR, RT_GROUP_CURSOR, RT_GROUP_ICON, RT_HTML, RT_ICON, RT_MANIFEST, RT_MENU, RT_MESSAGETABLE, RT_PLUGPLAY, RT_RCDATA, RT_STRING, RT_VERSION, RT_VXD, ResourceEntryIterator};
use memmap2::Mmap;
use parse::{le_u16, le_u32, take_vec};
use tracing::warn;

pub struct PeF {
    out: Vec<BufGenItm>,
    idx: usize,
}

impl GenFSProps for PeF {
    const FORMAT_NAME: &'static str = "pe";
}

impl PeF {
    pub fn walk_dir(pe: &PE, f: &FileRef, iter: &mut ResourceEntryIterator, path: String, out: &mut Vec<BufGenItm>) -> anyhow::Result<()> {
        let pe_align = pe.header.optional_header.unwrap().windows_fields.file_alignment;
        let resources_rva = pe.header.optional_header.unwrap().data_directories.get_resource_table().unwrap().virtual_address;
        let resource_offset = goblin::pe::utils::find_offset(resources_rva as usize, &pe.sections, pe_align, &ParseOptions::default()).unwrap();

        for e in iter {
            let e = e?;

            let name = if e.name_is_string() {
                let (i, len) = le_u16(&f.mmap[(resource_offset as usize + e.name_offset() as usize)..])?;
                let (_, utf16) = take_vec(i, len as usize, le_u16)?;
                let s = String::from_utf16_lossy(&utf16);
                s
            } else {
                rt_to_name(e.id().unwrap())
            };

            if e.data_is_directory() {
                // println!("dir - {name}");

                let dir_rva = resources_rva + e.offset_to_directory();
                let dir = ImageResourceDirectory::parse(
                    &f.mmap,
                    DataDirectory {
                        virtual_address: dir_rva,
                        size: 0,
                    },
                    &pe.sections,
                    pe_align,
                )?;

                let dir_offset = goblin::pe::utils::find_offset(dir_rva as usize, &pe.sections, pe_align, &ParseOptions::default()).unwrap();
                let mut e = dir.next_iter(dir_offset + std::mem::size_of::<ImageResourceDirectory>(), &f.mmap)?;

                Self::walk_dir(pe, f, &mut e, format!("{}/{}/", path, name), out)?;

            } else {
                // println!("data - {name}");
                let data_ent_offset = resource_offset + e.offset_to_data().unwrap() as usize;
                let (i, data_rva) = le_u32(&f.mmap[data_ent_offset..])?;
                let (_, data_sz) = le_u32(i)?;

                let data_offset = goblin::pe::utils::find_offset(data_rva as usize, &pe.sections, pe_align, &ParseOptions::default()).unwrap();
                out.push(BufGenItm::new(format!("{path}/{name}"), f.mmap[data_offset..][..data_sz as usize].to_vec()));
            }
        }
        Ok(())
    }
}

impl GenFS for PeF {
    fn try_open_internal(f: &FileRef) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let pe = PE::parse(f.mmap)?;


        let mut out = Vec::new();
        if let Some(res) = pe.resource_data {
            Self::walk_dir(&pe, f, &mut res.entries(), "_resources/".to_string(), &mut out)?;
        }

        for s in pe.sections {
            let dat = s.data(&f.mmap)?;
            let s_name = s.name()?;
            let name = format!("_section/{}", s_name);

            if let Some(dat) = dat {
                if s_name == "_winzip_" {
                    // Winzip puts the data at +0x80 bytes into a section _winzip_ but the zip has some of the offsets adjusted to be absolute to the rest of the PE.
                    // Normal extraction *might* work, I recommend using 7z on the original PE if this causes issues.
                    warn!("This PE contains data packed with WINZIP, this requires special handling. If you have problems use 7z to extract this.");

                    out.push(BufGenItm::new("_section/__winzip_data.zip".to_string(), dat[0x80..].to_vec()))
                }

                out.push(BufGenItm::new(&name, dat.to_vec()))
            } else {
                out.push(BufGenItm::new_empty(name));
            }
        }

        Ok(Self { out, idx: 0 })
    }

    fn sniff(f: &Mmap) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        Ok(PE::parse(f).is_ok())
    }

    fn next_itm(&mut self) -> anyhow::Result<Option<Box<dyn GenItem>>> {
        if let Some(resource) = self.out.get(self.idx) {
            self.idx += 1;
            return Ok(Some(Box::new(resource.clone())));
        }

        Ok(None)
    }

    fn name(&self) -> &str {
        Self::FORMAT_NAME
    }
}

fn rt_to_name(id: u16) -> String {
    match id {
        RT_CURSOR => "RT_CURSOR".to_string(),
        RT_BITMAP => "RT_BITMAP".to_string(),
        RT_ICON => "RT_ICON".to_string(),
        RT_MENU => "RT_MENU".to_string(),
        RT_DIALOG => "RT_DIALOG".to_string(),
        RT_STRING => "RT_STRING".to_string(),
        RT_FONTDIR => "RT_FONTDIR".to_string(),
        RT_FONT => "RT_FONT".to_string(),
        RT_ACCELERATOR => "RT_ACCELERATOR".to_string(),
        RT_RCDATA => "RT_RCDATA".to_string(),
        RT_MESSAGETABLE => "RT_MESSAGETABLE".to_string(),
        RT_GROUP_CURSOR => "RT_GROUP_CURSOR".to_string(),
        RT_GROUP_ICON => "RT_GROUP_ICON".to_string(),
        RT_VERSION => "RT_VERSION".to_string(),
        RT_DLGINCLUDE => "RT_DLGINCLUDE".to_string(),
        RT_PLUGPLAY => "RT_PLUGPLAY".to_string(),
        RT_VXD => "RT_VXD".to_string(),
        RT_ANICURSOR => "RT_ANICURSOR".to_string(),
        RT_ANIICON => "RT_ANIICON".to_string(),
        RT_HTML => "RT_HTML".to_string(),
        RT_MANIFEST => "RT_MANIFEST".to_string(),
        _ => format!("RT_{id}"),
    }
}
