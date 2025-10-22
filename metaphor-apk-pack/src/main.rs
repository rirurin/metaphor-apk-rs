// Accepts the following:
// An APK file, which creates a folder containing extracted DDS files + FileList.txt
// A folder containing DDS files + FileList.txt, compressed into an APK file

use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::path::{Path, PathBuf};
use metaphor_apk_rs::read::ApkReader;
use metaphor_apk_rs::serial::CompressionType;
use metaphor_apk_rs::write::ApkWriter;

#[derive(Debug)]
pub enum AppError {
    PrintUsage,
    PathDoesNotExist(String),
    WrongFileType,
    MissingFileList,
    UnknownCompressionType(String)
}

impl Error for AppError {}
impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PrintUsage => {
                write!(f,"Usage instructions:\n\
\n\
APK Mode: ./metaphor-apk-pack [input APK] (output)\n\
Input: APK to extract files from\n\
Output (optional): Where to save the extracted folder to.\n\
\n\
DDS Folder Mode: ./metaphor-apk-pack [input folder] (compression) (output)\n\
Input: A path to a folder containing one or more DDS files and FileList.txt\n\
Compression (optional): Define the compression algorithm used.\n\
(Valid options are Zlib, LZ4 and ZStd. LZ4 is used by default)\n\
Note that ZStd can only be used if your mod has a dependency set with OpenGFD\n\
Output (optional): A path to the folder where the output APK will be created")
            },
            _ => <Self as Debug>::fmt(self, f)
        }
    }
}

fn main() {
    if let Err(e) = app() {
        println!("{}", e);
    }
}

fn app() -> Result<(), Box<dyn Error>> {
    // handle CLI args
    let args: Vec<String> = std::env::args().enumerate()
        .filter_map(|(i, a)| if i > 0 { Some(a) } else { None }).collect();
    if args.len() < 1 {
        return Err(Box::new(AppError::PrintUsage));
    }
    let path = Path::new(&args[0]);
    if !std::fs::exists(path)? {
        return Err(Box::new(AppError::PathDoesNotExist(args[0].clone())));
    }
    let meta = std::fs::metadata(path)?;
    let out_idx = if meta.is_file() { 1 } else { 2 };
    let output = match args.len() > out_idx {
        true => PathBuf::from(&args[out_idx]),
        false => PathBuf::from(path.parent().unwrap().join(path.file_stem().unwrap()))
    };
    if meta.is_file() {
        // APK mode
        if path.extension().ok_or(Box::new(AppError::WrongFileType))? != "apk" {
            return Err(Box::new(AppError::WrongFileType));
        }
        let mut apk = ApkReader::read(path)?;
        if !std::fs::exists(&output)? {
            std::fs::create_dir(&output)?;
        }
        for (name, bytes) in apk.get_all_files()? {
            println!("Write to {:?}: {} bytes", output.join(name), bytes.len());
            std::fs::write(output.join(name), bytes.as_slice())?;
        }
        std::fs::write(output.join("FileList.txt"), apk.create_file_list())?;
    } else {
        // DDS folder mode
        let compression = match args.len() > 1 {
            true => {
                let cmp_str = (&args[1]).to_lowercase();
                match cmp_str.as_ref() {
                    "zlib" => CompressionType::ZLib,
                    "lz4" => CompressionType::LZ4,
                    "zstd" => CompressionType::ZStandard,
                    _ => return Err(Box::new(AppError::UnknownCompressionType(cmp_str)))
                }
            },
            false => CompressionType::LZ4
        };
        let file_list = path.join("FileList.txt");
        if !std::fs::exists(&file_list)? {
            return Err(Box::new(AppError::MissingFileList));
        }

        let out_path = match output.extension() {
            Some(_) => PathBuf::from(output),
            None => output.join(format!("{}.apk", path.file_stem().unwrap().to_str().unwrap()))
        };
        println!("Saving to \"{}\"", out_path.to_str().unwrap());
        let mut apk = ApkWriter::setup(out_path)?;
        let file_list = std::fs::read_to_string(&file_list)?;
        for entry in file_list.lines() {
            apk.add_external_file_with_compression(compression, &path.join(entry))?;
        }
        apk.save()?;
    }
    Ok(())
}
