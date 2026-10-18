extern crate core;

use anyhow::Context;
use clap::{Parser, Subcommand};
use std::fmt::Debug;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Parser)]
struct ExtCmd {
    /// The file to open
    #[clap(required = true)]
    input_path: String,

    #[clap(required = false, long = "fmt")]
    format_override: Option<String>,

    #[clap(required = false)]
    out_path: Option<String>,

    #[clap(required = false, long, short)]
    recurse: bool,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Mount the given file with FUSE
    #[cfg(all(target_os = "linux", feature = "with-fuse"))]
    Mnt {
        /// The file to open
        #[clap(required = true)]
        pth: String,

        #[clap(required = true)]
        src: String,
    },

    /// Extract the given file
    Ext(ExtCmd),

    /// List the contents of the given archive
    Ls {
        /// The file to open
        #[clap(required = true)]
        pth: String,

        #[clap(required = false, long)]
        fmt: Option<String>,
    },

    /// Identify the type of this file
    Id {
        /// The file to open
        #[clap(required = true)]
        pth: String,

        #[clap(required = false, long)]
        fmt: Option<String>,
    },

    /// Open the GUI to explore an archive
    Browse {
        /// The file to open
        #[clap(required = true)]
        pth: String,
    },

    /// List the supported formats
    Formats,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Try and parse in long format
    // If that fails try to parse just as an extraction
    // If that fails then reparse as long which will fail and print help
    let args = Args::try_parse();
    let args = args.unwrap_or_else(|_| match ExtCmd::try_parse() {
        Ok(v) => {
            info!("No subcommand, assuming 'extract {}'", v.input_path);
            Args { cmd: Cmd::Ext(v) }
        },
        Err(_) => {
            Args::parse();
            unreachable!();
        }
    });

    match args.cmd {
        #[cfg(all(target_os = "linux", feature = "with-fuse"))]
        Cmd::Mnt { pth: path_to_mount_to, src: file_to_mount } => {
            if !Path::new(&file_to_mount).is_file() {
                error!("The file you are trying to mount isn't a file!");
                return Ok(());
            }
            if !Path::new(&path_to_mount_to).is_dir() {
                error!("The path you are trying to mount to isn't a directory!");
                return Ok(());
            }

            // Do this first so it doesn't also have the signal handler
            let t = std::thread::spawn(move || {
                fuse::fuse_mnt::fuse_mount(path_to_mount_to, file_to_mount);
            });

            // Scan our child processes mounts, look for any fuse ones and abort them so we don't hang
            ctrlc::set_handler(move || {
                for task in std::fs::read_dir("/proc/self/task").unwrap() {
                    let task = task.unwrap();

                    if task.file_name().to_string_lossy().contains(&format!("{}", unsafe { libc::getpid()})) {
                        continue;
                    }

                    debug!("{}", task.path().display());

                    let mi = std::fs::read_to_string(task.path().join("./mountinfo")).unwrap();

                    debug!("{}", mi);

                    let m = mi.lines().filter(|f| f.contains("- fuse /dev/fuse")).collect::<Vec<_>>();
                    for m in m {
                        debug!("-- {}", m);
                        let x = m.split(" ").nth(2).unwrap().to_string();
                        let num = x.split(":").nth(1).unwrap().to_string();
                        std::fs::write(PathBuf::from("/sys/fs/fuse/connections/").join(num).join("./abort"), b"1\n").unwrap();
                    }
                }
            }).context("Failed to set handler")?;

            t.join().unwrap();
        }
        Cmd::Formats => {
            for f in genericfs::generic_fs::FORMATS {
                println!("- {}", f.format_name());
            }
        }
        Cmd::Browse { pth } => {
            let pth = PathBuf::from(pth);
            let pth = std::fs::canonicalize(pth)?;

            if !pth.exists() {
                error!("Src file not found");
                return Ok(());
            }

            if let Some(f) = genericfs::generic_fs::try_open(&pth, None) {
                gui::run_gui(f, &pth);
            } else {
                error!("Failed to open file");
            }
        }
        Cmd::Ext(ext) => {
            let (pth, fmt, out) = (ext.input_path, ext.format_override, ext.out_path);
            let pth = PathBuf::from(pth);
            let pth = std::fs::canonicalize(pth).context("Failed to gen canonical path")?;

            if !pth.exists() {
                error!("Src file not found");
                return Ok(());
            }

            let cur_dir = std::env::current_dir()?;
            let out = out.map(PathBuf::from).unwrap_or(cur_dir.join("_ext"));

            extract_file(&pth, &out, &fmt, ext.recurse)?;
        }
        Cmd::Ls { pth, fmt } => {
            let pth = PathBuf::from(pth);
            let pth = std::fs::canonicalize(pth)?;

            if !pth.exists() {
                error!("Src file not found");
                return Ok(());
            }

            if let Some(mut f) = genericfs::generic_fs::try_open(&pth, fmt.as_ref()) {
                info!("Found a {} file", f.name());
                while let Ok(Some(fi)) = f.next_itm() {
                    info!("- '{}', {}k", fi.name(), fi.size() >> 12,);
                }
            } else {
                error!("Unrecognized format");
            }
        }
        Cmd::Id { pth, fmt } => {
            let pth = PathBuf::from(pth);
            let pth = std::fs::canonicalize(pth)?;

            if !pth.exists() {
                error!("Src file not found");
                return Ok(());
            }

            if let Some(f) = genericfs::generic_fs::try_open(&pth, fmt.as_ref()) {
                info!("Found a {} file", f.name());
            } else {
                error!("Unrecognized format");
            }
        }
    }

    Ok(())
}

pub fn extract_file(
    pth: &Path,
    out: &Path,
    fmt: &Option<String>,
    recurse: bool,
) -> anyhow::Result<()> {
    info!("Extracting {} to {}", pth.display(), out.display());

    if let Some(mut f) = genericfs::generic_fs::try_open(pth, fmt.as_ref()) {
        if !out.exists() {
            info!("Out dir not present, creating {}", out.display());
            let _ = std::fs::create_dir_all(out);
        } else {
            info!("Out already exists");
        }

        info!("Found a {} file", f.name());
        while let Ok(Some(mut fi)) = f.next_itm() {
            let out2 = out.to_path_buf();

            let output_file = out2.to_owned().join(Path::new(&fi.name()));

            if !std::path::absolute(&output_file)?.starts_with(std::path::absolute(&out2)?) {
                warn!("Writing outside of current, refusing");
                continue;
            }

            // handle slash paths
            std::fs::create_dir_all(output_file.parent().context("No parent")?)
                .context("Failed to create directory")?;

            if output_file == Path::new("./") {
                info!("Ignoring output self {}", output_file.display());
                continue;
            }

            info!(
                "Extracting '{}' of size {}k -> {}",
                fi.name(),
                fi.size() >> 12,
                output_file.display()
            );

            let pp = progression::Bar::new(fi.size(), progression::Config::cargo());
            let mut f = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(output_file)?;
            let mut buf = [0u8; 0x4000];
            loop {
                let p = fi.read(&mut buf);
                pp.inc(p);
                if p == 0 {
                    break;
                }
                f.write_all(&buf[..p as usize])?;
            }

            // let data = fi.read_to_vec();
            // std::fs::write(output_file, data).context(format!("Failed to extract {}", fi.name()))?;
        }
    }

    if recurse {
        fn recurse_dir(out: &Path, fmt: &Option<String>, recurse: bool) -> anyhow::Result<()> {
            for ent in std::fs::read_dir(out)
                .context("Failed to read dir")?
                .flatten()
            {
                if ent.file_type()?.is_dir() {
                    recurse_dir(&ent.path(), fmt, recurse)?;
                    continue;
                }

                let pth = ent.path();
                let out = out.join(format!("{}_ext", ent.file_name().to_string_lossy()));

                if let Err(e) = extract_file(&pth, &out, fmt, recurse) {
                    error!("Failed to extract {}, {}", pth.display(), e);
                }
            }

            Ok(())
        }

        recurse_dir(out, fmt, false)?;
    }

    Ok(())
}

#[cfg(all(target_os = "linux", feature = "with-fuse"))]
mod fuse;
pub mod gui;
