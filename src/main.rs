extern crate core;

use anyhow::Context;
use clap::{Parser, Subcommand};
use std::fmt::Debug;
use std::fs::OpenOptions;
use std::io::{Write};
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Parser)]
struct ExtCmd {
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
    #[cfg(target_os = "linux")]
    Mnt {
        #[clap(required = true)]
        pth: String,

        #[clap(required = true)]
        src: String,
    },
    Ext(ExtCmd),
    Ls {
        #[clap(required = true)]
        pth: String,

        #[clap(required = false, long)]
        fmt: Option<String>,
    },
    Browse {
        #[clap(required = true)]
        pth: String,
    },
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
        Ok(v) => Args { cmd: Cmd::Ext(v) },
        Err(_) => {
            Args::parse();
            unreachable!();
        }
    });

    match args.cmd {
        #[cfg(target_os = "linux")]
        Cmd::Mnt { pth, src } => {
            use std::io::{Read};
            std::thread::spawn(move || {
                fuse::fuse_mnt::fuse_mount(pth, src);
            });

            loop {
                let mut buf = [0u8; 1];
                std::io::stdin().read_exact(&mut buf)?;
                if buf[0] == b'a' {
                    break;
                }
                println!("---");
            }
        }
        Cmd::Browse { pth } => {
            let pth = PathBuf::from(pth);
            let pth = std::fs::canonicalize(pth)?;

            if !pth.exists() {
                error!("Src file not found");
                return Ok(());
            }

            if let Some(_f) = genericfs::generic_fs::try_open(&pth, None) {
                // gui::run_gui(f, &pth);
            } else {
                error!("Failed to open file");
            }
        }
        Cmd::Ext(ext) => {
            let (pth, fmt, out) = (ext.input_path, ext.format_override, ext.out_path);
            let pth = PathBuf::from(pth);
            let pth = std::fs::canonicalize(pth)?;

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
        for ent in std::fs::read_dir(out)
            .context("Failed to read dir")?
            .flatten()
        {
            if ent.file_type()?.is_dir() {
                info!("Ignoring dir {}", ent.path().display());
                continue;
            }

            let pth = ent.path();
            let out = out.join(format!("{}_ext", ent.file_name().to_string_lossy()));

            if let Err(e) = extract_file(&pth, &out, fmt, recurse) {
                error!("Failed to extract {}, {}", pth.display(), e);
            }
        }
    }

    Ok(())
}

#[cfg(target_os = "linux")]
mod fuse;
// pub mod gui;