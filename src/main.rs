use std::{env, io, path, process};

use walk_rs::{self, FileWalker};

fn main() -> io::Result<()> {
    // This application still needs to be made
    // more ergonomic in order to be used as a
    // generic CLI tool.
    // TODO: implement `clap` as the CLI
    // framework.
    let mut args = env::args();
    let _ = args.next();

    if args.len() < 1 {
        print_usage();
        eprintln!("error: missing {:?}", "path");
        process::exit(2)
    }

    let path = args
        .next()
        .unwrap()
        .parse::<path::PathBuf>()
        .expect("path parsed as PathBuf");

    if !path.is_dir() {
        print_usage();
        eprintln!("error: {:?} not a directory", path);
        process::exit(3)
    }

    // You'll want to look around the impl of this
    // struct to get an idea of what it's current
    // API looks like. While this is exposing the
    // gist of what it does, theres a bit more to
    // know about how filtering happens using
    // `Predicate` objects.
    FileWalker::new(path)
        .with_min_depth(0)
        .with_max_depth(4)
        .with_callback(|child| {
            println!("{child:?}");
        })
        .with_predicate(walk_rs::file_excludes_format!("application/text"))
        .with_predicate(walk_rs::file_excludes!("main.*"))
        .with_predicate(walk_rs::parent_excludes!("SCANS"))
        .walk()?;

    Ok(())
}

fn print_usage() {
    let mut args = env::args();
    eprintln!("{} <PATH>", args.next().unwrap());
}
