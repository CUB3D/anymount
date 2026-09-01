use std::path::{Path, PathBuf};

use genericfs::generic_fs::GenFS;
use iced::{
    Task, Theme,
    widget::{Container, container, scrollable},
};
use rfd::FileDialog;
use tracing::error;

use iced::widget::{button, column, text};

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Browse(usize),
    Back,
    Extract(usize),
    View(usize),
}

#[derive(Clone)]
struct GenItem2 {
    name: String,
    size: u64,
    data: Vec<u8>,
}

#[derive(Clone)]
pub struct OpenFile {
    kind: String,
    path: String,
    items: Vec<GenItem2>,
}

#[derive(Clone)]
struct GuiRoot {
    pub stack: Vec<OpenFile>,
}

impl GuiRoot {
    pub fn view(&self) -> Container<'_, Message> {
        let cur = self.stack.last().unwrap();

        let mut c = column![
            text(format!("Contents of '{}'", cur.path)),
            text(format!("kind = {}", cur.kind)),
        ];

        if self.stack.len() > 1 {
            c = c.push(button("Back").on_press(Message::Back));
        }

        c = c.push(iced::widget::table(
            [
                iced::widget::table::column("Name", |(_, file): (usize, &GenItem2)| {
                    text(&file.name)
                }),
                iced::widget::table::column("Size", |(_, file): (usize, &GenItem2)| {
                    let mut size = format!("{} b", file.size);
                    if file.size > 1024 {
                        size = format!("{} Kib", file.size / 1024);
                    }
                    if file.size > (1024 * 1024) {
                        size = format!("{} Mib", file.size / (1024 * 1024));
                    }
                    if file.size > (1024 * 1024 * 1024) {
                        size = format!("{} Gib", file.size / (1024 * 1024 * 1024));
                    }

                    text(size)
                }),
                iced::widget::table::column("Browse", |(idx, _): (usize, &GenItem2)| {
                    button("Browse").on_press(Message::Browse(idx))
                }),
                iced::widget::table::column("Extract", |(idx, _): (usize, &GenItem2)| {
                    button("Extract").on_press(Message::Extract(idx))
                }),
                iced::widget::table::column("View", |(idx, _): (usize, &GenItem2)| {
                    button("View").on_press(Message::View(idx))
                }),
            ],
            self.stack.last().unwrap().items.iter().enumerate(),
        ));

        container(scrollable(c)).padding(10)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        let cur = self.stack.last().unwrap();
        match message {
            Message::Extract(idx) => {
                let f = cur.items[idx].clone();
                if let Some(dest) = FileDialog::new().set_file_name(f.name).save_file() {
                    std::fs::write(&dest, &f.data).unwrap();
                }

                Task::none()
            }
            Message::View(idx) => {
                let f = cur.items[idx].clone();
                std::fs::write("./temp.bin", &f.data).unwrap();
                #[expect(clippy::zombie_processes)]
                std::process::Command::new("kwrite")
                    .arg("./temp.bin")
                    .spawn()
                    .unwrap();

                Task::none()
            }
            Message::Back => {
                self.stack.pop();
                Task::none()
            }
            Message::Browse(idx) => {
                let f = cur.items[idx].clone();
                let pth = PathBuf::from("./test.bin");
                std::fs::write(&pth, &f.data).unwrap();
                if let Some(mut fs) = genericfs::generic_fs::try_open(&pth, None) {
                    let mut items = Vec::new();
                    while let Ok(Some(mut i)) = fs.next_itm() {
                        items.push(GenItem2 {
                            data: i.read_to_vec(),
                            name: i.name().to_string(),
                            size: i.size(),
                        });
                    }

                    let new = OpenFile {
                        items,
                        kind: fs.name().to_string(),
                        path: format!("{}::{}", self.stack.last().unwrap().path, f.name),
                    };
                    self.stack.push(new);
                } else {
                    error!("Failed to open file");
                }

                Task::none()
            }
        }
    }
}

pub fn run_gui(mut fs: Box<dyn GenFS>, pth: &Path) {
    let mut items = Vec::new();
    while let Ok(Some(mut i)) = fs.next_itm() {
        items.push(GenItem2 {
            data: i.read_to_vec(),
            name: i.name().to_string(),
            size: i.size(),
        });
    }

    let dialog = GuiRoot {
        stack: vec![OpenFile {
            items,
            kind: fs.name().to_string(),
            path: pth.to_str().unwrap().to_string(),
        }],
    };

    let _ = iced::application(move || dialog.clone(), GuiRoot::update, GuiRoot::view)
        .theme(Theme::Dark)
        .centered()
        .run();
}
