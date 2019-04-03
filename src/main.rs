use rayon::prelude::*;
use sha2::{Digest, Sha512};
use std::fs::File;
use std::io::{BufRead, Read};
use std::os::linux::fs::MetadataExt;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Options {
    #[structopt(
        long = "src_file",
        parse(from_os_str),
        help = "a file in which path of files that we want to check cheating are.",
        long_help = "a file in which path of files that we want to check cheating are.
if it is not presented, the paths is read from stdin."
    )]
    src_file: Option<PathBuf>,

    #[structopt(
        parse(from_os_str),
        help = "a file in which path of files that will be compared to source files are."
    )]
    dest_file: PathBuf,
}

#[derive(Clone, Debug)]
struct FileData {
    path: String,
    inode: u64,
}

#[derive(Clone, Debug)]
struct HomeworkData {
    meta: FileData,
    hash_val: sha2::digest::generic_array::GenericArray<u8, <sha2::Sha512 as Digest>::OutputSize>,
}

enum SourceType {
    File(std::fs::File),
    Stdin(std::io::Stdin),
}

impl Read for SourceType {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            SourceType::File(f) => f.read(buf),
            SourceType::Stdin(s) => s.read(buf),
        }
    }
}

fn read_homework_data<T: BufRead>(reader: T) -> Vec<HomeworkData> {
    reader
        .lines()
        .filter_map(|line| line.ok())
        .filter_map(|path| {
            File::open(&path).ok().map(|mut file| {
                let mut sha_hasher = Sha512::new();
                std::io::copy(&mut file, &mut sha_hasher).unwrap();

                HomeworkData {
                    meta: FileData {
                        path,
                        inode: file.metadata().unwrap().st_ino(),
                    },
                    hash_val: sha_hasher.result(),
                }
            })
        })
        .collect()
}

fn main() {
    let option = Options::from_args();

    let src = if let Some(file) = option.src_file.as_ref().and_then(|p| File::open(p).ok()) {
        SourceType::File(file)
    } else {
        SourceType::Stdin(std::io::stdin())
    };

    let dest = std::io::BufReader::new(
        File::open(option.dest_file).expect("dest_file is not a file path."),
    );
    let dest = read_homework_data(dest);

    let src = read_homework_data(std::io::BufReader::new(src));
    src.par_iter().for_each(|src| {
        rayon::iter::repeat(src)
            .zip(dest.par_iter())
            .filter(|(src, dest)| src.meta.inode != dest.meta.inode)
            .inspect(|(src, dest)| {
                eprintln!("check between '{}', '{}'", src.meta.path, dest.meta.path);
            })
            .filter(|(src, dest)| src.hash_val == dest.hash_val)
            .for_each(|(src, dest)| {
                println!("These are same: {}, {}", src.meta.path, dest.meta.path)
            });
    });
}
