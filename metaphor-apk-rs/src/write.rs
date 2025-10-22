use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;
use crate::serial::CompressionType;

#[derive(Debug)]
pub enum WriterError {
    FilePathMissing(String),
    FileNameMissing,
    FileAlreadyExists(String)
}

impl Error for WriterError {}
impl Display for WriterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        <Self as Debug>::fmt(self, f)
    }
}

pub struct ApkWriterEntry<'a> {
    index: usize,
    compression_type: CompressionType,
    data: Box<dyn Read + 'a>
}

impl<'a> ApkWriterEntry<'a> {
    pub fn new(index: usize, compression_type: CompressionType, data: Box<dyn Read + 'a>) -> Self {
        Self { index, compression_type, data }
    }
}

pub struct ApkWriter<'a, S: Write + Seek> {
    owner: S,
    // preserve order that files were inserted into APK in
    files: HashMap<String, ApkWriterEntry<'a>>
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
        self.add_external_file_with_compression(CompressionType::LZ4, path)
    }

    pub fn add_external_file_with_compression<P: AsRef<Path>>(&mut self,
        cmp_type: CompressionType, path: P) -> Result<(), Box<dyn Error>> {
        if !std::fs::exists(&path)? {
            return Err(Box::new(WriterError::FilePathMissing(path.as_ref().to_str().unwrap().to_string())));
        }
        let name = path.as_ref().file_name().ok_or(WriterError::FileNameMissing)?
            .to_str().unwrap().to_string();
        if self.files.contains_key(&name) {
            return Err(Box::new(WriterError::FileAlreadyExists(name)));
        }
        let stream = File::open(path)?;
        self.files.insert(name, ApkWriterEntry::new(self.files.len(), cmp_type, Box::new(stream)));
        Ok(())
    }

    pub fn add_internal_file(&mut self, name: &str, stream: &'a [u8]) -> Result<(), Box<dyn Error>> {
        self.add_internal_file_with_compression(name, CompressionType::LZ4, stream)
    }

    pub fn add_internal_file_with_compression(&mut self, name: &str,
        cmp_type: CompressionType, stream: &'a [u8]) -> Result<(), Box<dyn Error>> {
        let name = name.to_string();
        if self.files.contains_key(&name) {
            return Err(Box::new(WriterError::FileAlreadyExists(name)));
        }
        self.files.insert(name, ApkWriterEntry::new(self.files.len(), cmp_type, Box::new(stream)));
        Ok(())
    }

    pub fn remove_file(&mut self, name: &str) -> Option<ApkWriterEntry<'a>> {
        self.files.remove(name)
    }

    pub fn save(&mut self) -> Result<(), Box<dyn Error>> {
        self.owner.write(crate::serial::Header::new(self.files.len()).to_bytes())?;
        let blank = [0u8; 0x100];
        let mut pointer = (self.files.len() * size_of::<crate::serial::FileHeader>())
            + size_of::<crate::serial::Header>();
        let mut files = Vec::with_capacity(self.files.len());
        (0..self.files.len()).for_each(|_| files.push(None));
        for (name, entry) in &mut self.files {
            let index = entry.index;
            files[index] = Some((name, entry));
        }
        for (i, (name, entry)) in files.iter_mut()
            .filter_map(|e| e.as_mut()).enumerate() {
            // get file contents
            let mut file = vec![];
            entry.data.read_to_end(&mut file)?;
            // compress file
            let (cmp_real_size, cmp_pad_size, compressed) = match entry.compression_type {
                CompressionType::ZLib => {
                    let mut compressed = vec![];
                    let cmp_real_size = {
                        let mut encoder = flate2::write::ZlibEncoder::new(&mut compressed, flate2::Compression::fast());
                        encoder.write_all(&file)?;
                        encoder.finish()?.len()
                    };
                    let cmp_pad_size = (cmp_real_size + 0xf) & !0xf; // align to nearest 0x10
                    (cmp_real_size, cmp_pad_size, compressed)
                },
                CompressionType::LZ4 => {
                    #[cfg(feature = "use-lz4-flex")]
                    {
                        let max_possible_size = (lz4_flex::block::get_maximum_output_size(file.len()) + 0xf) & !0xf;
                        let mut compressed = Vec::with_capacity(max_possible_size);
                        unsafe { compressed.set_len(compressed.capacity()) };
                        let cmp_real_size = lz4_flex::block::compress_into(&file, &mut compressed)?;
                        unsafe { compressed.set_len(cmp_real_size) };
                        let cmp_pad_size = (cmp_real_size + 0xf) & !0xf; // align to nearest 0x10
                        (cmp_real_size, cmp_pad_size, compressed)
                    }
                    #[cfg(feature = "use-lz4")]
                    {
                        let max_possible_size = unsafe { lz4::liblz4::LZ4F_compressBound(file.len(), std::ptr::null()) as usize & (isize::MAX as usize) };
                        let mut compressed = Vec::with_capacity(max_possible_size);
                        unsafe { compressed.set_len(compressed.capacity()) };
                        let cmp_real_size = lz4::block::compress_to_buffer(&file, None, false, &mut compressed)?;
                        unsafe { compressed.set_len(cmp_real_size) };
                        let cmp_pad_size = (cmp_real_size + 0xf) & !0xf; // align to nearest 0x10
                        (cmp_real_size, cmp_pad_size, compressed)
                    }
                },
                CompressionType::ZStandard => {
                    let compressed = zstd::encode_all(std::io::Cursor::new(&file), zstd::DEFAULT_COMPRESSION_LEVEL)?;
                    let cmp_pad_size = (compressed.len() + 0xf) & !0xf; // align to nearest 0x10
                    (compressed.len(), cmp_pad_size, compressed)
                },
            };
            self.owner.write(crate::serial::FileHeader::new(name, cmp_pad_size, pointer).to_bytes())?;
            self.owner.seek(SeekFrom::Start(pointer as u64))?;
            self.owner.write(crate::serial::DataHeader::new(cmp_real_size,
                entry.compression_type, file.len()).to_bytes())?;
            self.owner.write(&compressed)?;
            if cmp_real_size % 0x10 != 0 { // fill padding with zeroes
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

    #[test]
    fn test_write() -> Result<(), Box<dyn Error>> {
        let mut apk = ApkWriter::setup("E:/Metaphor/base_cpk/COMMON/ui/ss/01_grandtrad_out.apk")?;
        apk.add_external_file("E:/Metaphor/base_cpk/COMMON/ui/ss/01_grandtrad.dds")?;
        apk.save()?;
        Ok(())
    }
}