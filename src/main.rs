#![deny(unsafe_code)]

use camino::Utf8PathBuf;
use clap::Parser;
use ignore::WalkBuilder;
use owo_colors::OwoColorize;
use ptree::{print_tree, TreeBuilder};
use std::process::ExitCode;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Directory to display
    #[clap(default_value = ".")]
    dir: Utf8PathBuf,

    /// Maximum depth to recurse into directories
    #[clap(short, long)]
    depth: Option<usize>,

    /// Display hidden files
    #[clap(short = 'H', long)]
    hidden: bool,

    /// Ignore errors
    #[clap(short, long)]
    ignore_errors: bool,

    /// Do not respect .gitignore files
    #[clap(short, long)]
    no_ignore: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();

    let walker = WalkBuilder::new(&args.dir)
        .ignore(!args.no_ignore)
        .git_ignore(!args.no_ignore)
        .git_global(!args.no_ignore)
        .max_depth(args.depth)
        .hidden(!args.hidden)
        .sort_by_file_name(|a, b| a.cmp(b))
        .skip_stdout(true)
        .build();

    let mut tree = TreeBuilder::new(args.dir.to_string());

    let mut current_depth: usize = 1;

    let mut files_count: usize = 0;
    let mut dirs_count: usize = 0;
    let mut symlink_count: usize = 0;

    for entry in walker.skip(1) {
        match entry {
            Ok(entry) => {
                let entry_depth = entry.depth();

                if entry_depth < current_depth {
                    for _ in 0..(current_depth - entry_depth) {
                        tree.end_child();
                    }

                    current_depth = entry_depth;
                }

                match entry.file_type() {
                    Some(file_type) => {
                        let file_type = file_type.into();

                        let file_name = entry.file_name().to_string_lossy();

                        match file_type {
                            FileType::Directory => {
                                tree.begin_child(format!("{}/", file_name.green()));

                                current_depth += 1;

                                dirs_count += 1;
                            }
                            FileType::File => {
                                tree.add_empty_child(file_name.into());

                                files_count += 1;
                            }
                            FileType::Symlink => {
                                tree.add_empty_child(format!(
                                    "{} -> {}",
                                    file_name.blue(),
                                    match entry.path().read_link() {
                                        Ok(s) => s,
                                        Err(err) => {
                                            if !args.ignore_errors {
                                                eprintln!("{}", err);
                                            }
                                            continue;
                                        }
                                    }
                                    .to_string_lossy()
                                    .cyan()
                                ));

                                files_count += 1;
                                symlink_count += 1;
                            }
                            FileType::Other => {
                                tree.add_empty_child(format!("{}", file_name.red()));

                                files_count += 1;
                            }
                        }
                    }
                    None => continue,
                }
            }
            Err(err) => {
                if !args.ignore_errors {
                    eprintln!("{}", err);
                }
            }
        }
    }

    print_tree(&tree.build())
        .map_err(|err| {
            eprintln!("{}", err);
        })
        .ok();

    println!(
        "\n{} directories, {} files{}",
        dirs_count,
        files_count,
        if symlink_count > 0 {
            format!(", {} symlinks", symlink_count)
        } else {
            "".to_string()
        }
    );

    ExitCode::SUCCESS
}

#[derive(Debug)]
enum FileType {
    Directory,
    File,
    Symlink,
    Other,
}

impl From<std::fs::FileType> for FileType {
    fn from(file_type: std::fs::FileType) -> Self {
        if file_type.is_dir() {
            Self::Directory
        } else if file_type.is_file() {
            Self::File
        } else if file_type.is_symlink() {
            Self::Symlink
        } else {
            Self::Other
        }
    }
}
