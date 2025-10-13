use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;

#[derive(Debug)]
pub enum WriterError {
    FileNameMissing,
    FileAlreadyExists(String)
}

impl Error for WriterError {}
impl Display for WriterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        <Self as Debug>::fmt(self, f)
    }
}

pub struct ApkWriter<'a, S: Write + Seek> {
    owner: S,
    files: HashMap<String, Box<dyn Read + 'a>>
}

impl ApkWriter<'_, BufWriter<File>> {
    pub fn setup<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let owner = BufWriter::new(File::create(path)?);
        let files = HashMap::new();
        Ok(Self {
            owner, files
        })
    }
}

impl<'a, S: Write + Seek> ApkWriter<'a, S> {
    pub fn add_external_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Box<dyn Error>> {
        let name = path.as_ref().file_name().ok_or(WriterError::FileNameMissing)?
            .to_str().unwrap().to_string();
        if self.files.contains_key(&name) {
            return Err(Box::new(WriterError::FileAlreadyExists(name)));
        }
        let stream = File::open(path)?;
        self.files.insert(name, Box::new(stream));
        Ok(())
    }

    pub fn add_internal_file(&mut self, name: &str, stream: &'a [u8]) -> Result<(), Box<dyn Error>> {
        let name = name.to_string();
        if self.files.contains_key(&name) {
            return Err(Box::new(WriterError::FileAlreadyExists(name)));
        }
        self.files.insert(name, Box::new(stream));
        Ok(())
    }

    pub fn save(&mut self) -> Result<(), Box<dyn Error>> {
        self.owner.write(crate::serial::Header::new(self.files.len()).to_bytes())?;
        let blank = [0u8; 0x100];
        let mut pointer = (self.files.len() * size_of::<crate::serial::FileHeader>())
            + size_of::<crate::serial::Header>();
        for (i, (name, data)) in
            self.files.iter_mut().enumerate() {
            // get file contents
            let mut file = vec![];
            data.read_to_end(&mut file)?;
            // compress file
            let max_possible_size = (lz4_flex::block::get_maximum_output_size(file.len()) + 0xf) & !0xf;
            let mut compressed = Vec::with_capacity(max_possible_size);
            unsafe { compressed.set_len(compressed.capacity()) };
            let cmp_real_size = lz4_flex::block::compress_into(&file, &mut compressed)?;
            unsafe { compressed.set_len(cmp_real_size) };
            // align to nearest 0x10
            let cmp_pad_size = (cmp_real_size + 0xf) & !0xf;
            self.owner.write(crate::serial::FileHeader::new(name, cmp_pad_size, pointer).to_bytes())?;
            self.owner.seek(SeekFrom::Start(pointer as u64))?;
            self.owner.write(crate::serial::DataHeader::new(cmp_real_size, file.len()).to_bytes())?;
            self.owner.write(&compressed)?;
            // fill padding with zeroes
            if cmp_real_size % 0x10 != 0 {
                self.owner.write(&blank[..0x10 - (cmp_real_size % 0x10)])?;
            }
            pointer += cmp_pad_size + size_of::<crate::serial::DataHeader>();
            let next_file_header = size_of::<crate::serial::Header>()
            + ((i + 1) * size_of::<crate::serial::FileHeader>());
            self.owner.seek(SeekFrom::Start(next_file_header as u64))?;
        }
        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use std::error::Error;
    use crate::write::ApkWriter;

    /*
    #[test]
    fn test_write() -> Result<(), Box<dyn Error>> {
        let mut apk = ApkWriter::setup("E:/Metaphor/base_cpk/COMMON/ui/ss/01_grandtrad_out.apk")?;
        apk.add_external_file("E:/Metaphor/base_cpk/COMMON/ui/ss/01_grandtrad.dds")?;
        apk.save()?;
        Ok(())
    }
    */
}