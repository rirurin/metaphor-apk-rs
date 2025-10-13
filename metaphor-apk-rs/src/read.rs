use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::mem::MaybeUninit;
use std::path::Path;
use crate::serial::{DataHeader, FileHeader, Header};

#[derive(Debug)]
pub enum ReaderError {
    FileNotFound(String)
}

impl Error for ReaderError {}
impl Display for ReaderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        <Self as Debug>::fmt(self, f)
    }
}

pub struct ApkReader<S: Read + Seek> {
    owner: S,
    files: Vec<FileHeader>
}

impl ApkReader<BufReader<File>> {
    pub fn read<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let mut owner = BufReader::new(File::open(path)?);
        let mut header: MaybeUninit<Header> = MaybeUninit::uninit();
        owner.read(unsafe { &mut *(header.as_mut_ptr() as *mut [u8; size_of::<Header>()]) })?;
        let header = unsafe { header.assume_init() };
        let mut files = Vec::with_capacity(header.count as usize);
        let head_area = unsafe { std::slice::from_raw_parts_mut(
            files.as_mut_ptr() as *mut u8, header.count as usize * size_of::<FileHeader>()) };
        owner.read(head_area)?;
        unsafe { files.set_len(header.count as usize) };
        Ok(Self { owner, files })
    }
}

impl<S: Read + Seek> ApkReader<S> {
    pub fn get_file_inner(owner: &mut S, f: &FileHeader) -> Result<Vec<u8>, Box<dyn Error>> {
        // get data header
        owner.seek(SeekFrom::Start(f.offset as u64))?;
        let mut data_header: MaybeUninit<DataHeader> = MaybeUninit::uninit();
        owner.read(unsafe { &mut *(data_header.as_mut_ptr() as *mut [u8; size_of::<DataHeader>()]) })?;
        let data_header = unsafe { data_header.assume_init() };
        // decompress using LZ4
        let mut compressed = Vec::with_capacity(data_header.compressed as usize);
        unsafe { compressed.set_len(data_header.compressed as usize) };
        let mut bytes_read = 0;
        while bytes_read < data_header.compressed as usize {
            bytes_read += owner.read(&mut compressed.as_mut_slice()[bytes_read..])?;
        }
        let mut out = Vec::with_capacity(data_header.decompressed as usize);
        unsafe { out.set_len(data_header.decompressed as usize) };
        lz4_flex::block::decompress_into(compressed.as_slice(), out.as_mut_slice())?;
        Ok(out)
    }

    pub fn get_file(&mut self, name: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        for f in &self.files {
            if f.get_filename() == name {
                return Self::get_file_inner(&mut self.owner, f);
            }
        }
        Err(Box::new(ReaderError::FileNotFound(name.to_string())))
    }

    pub fn get_all_files(&mut self) -> Result<HashMap<&str, Vec<u8>>, Box<dyn Error>> {
        let mut files = HashMap::new();
        for f in &self.files {
            files.insert(f.get_filename(),Self::get_file_inner(&mut self.owner, f)?);
        }
        Ok(files)
    }
}

#[cfg(test)]
pub mod tests {
    use std::error::Error;
    use crate::read::ApkReader;

    #[test]
    fn test_read() -> Result<(), Box<dyn Error>> {
        let path = "E:/Metaphor/base_cpk/COMMON/ui/ss/01_grandtrad.apk";
        if !std::fs::exists(path)? {
            return Ok(());
        }
        let mut apk = ApkReader::read(path)?;
        let file = apk.get_file("01_grandtrad.dds")?;
        println!("{}", file.len());
        std::fs::write("E:/Metaphor/base_cpk/COMMON/ui/ss/01_grandtrad.dds", &file)?;
        Ok(())
    }
}