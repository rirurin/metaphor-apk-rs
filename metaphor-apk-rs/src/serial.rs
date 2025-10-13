use std::ffi::CStr;
use std::fmt::{Debug, Formatter};

#[repr(C)]
#[derive(Debug)]
pub struct Header {
    magic: [u8; 6],
    field6: u16,
    pub(crate) count: u32,
    reserve: u32
}

pub(crate) static APK_MAGIC: [u8; 6] = [0x50, 0x41, 0x43, 0x4b, 0, 0];

impl Header {
    pub fn check_magic(&self) -> bool {
        self.magic == APK_MAGIC
    }

    pub fn new(count: usize) -> Self {
        Self {
            magic: APK_MAGIC,
            field6: 1,
            count: count as u32,
            reserve: 0
        }
    }

    pub fn to_bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(&raw const *self as _, size_of::<Self>()) }
    }
}

const _: () = {
    ["Size of APK Header"][size_of::<Header>() - 0x10];
};

#[repr(C)]
pub struct FileHeader {
    filename: [i8; 0x100],
    pub(crate) file_size: u32,
    unk: [u32; 5],
    pub(crate) offset: u32,
    unk2: u32
}

impl FileHeader {
    pub fn get_filename(&self) -> &str {
        unsafe { CStr::from_ptr(self.filename.as_ptr()).to_str().unwrap() }
    }

    pub fn new(name: &str, file_size: usize, offset: usize) -> Self {
        let mut filename = [0; 0x100];
        unsafe { std::ptr::copy_nonoverlapping(name.as_ptr() as _, filename.as_mut_ptr(), name.len()) };
        Self {
            filename,
            file_size: (file_size + size_of::<DataHeader>()) as u32,
            unk: [0; 5],
            offset: offset as u32,
            unk2: 0
        }
    }

    pub fn to_bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(&raw const *self as _, size_of::<Self>()) }
    }
}

impl Debug for FileHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, " FileHeader {{ name: {}, file_size: {}, offset: {} }}",
            self.get_filename(), self.file_size, self.offset)
    }
}

const _: () = {
    ["Size of file header"][size_of::<FileHeader>() - 0x120];
};

#[repr(C, align(16))]
#[derive(Debug)]
pub struct DataHeader {
    magic: u32,
    bitfield: u32,
    f8: u32,
    pub(crate) decompressed: u32,
    pub(crate) length: u32,
    unk: [u32; 3],
    pub(crate) compressed: u32,
    pub(crate) header_size: u32
}

pub(crate) static APK_DATA_MAGIC: u32 = 0x305a5a5a;

impl DataHeader {
    pub fn check_magic(&self) -> bool {
        self.magic == APK_DATA_MAGIC
    }

    pub fn new(cmp_size: usize, dcmp_size: usize) -> Self {
        Self {
            magic: APK_DATA_MAGIC,
            bitfield: 0x010001,
            f8: 0,
            decompressed: dcmp_size as u32,
            length: (cmp_size + size_of::<Self>()) as u32,
            unk: [0; 3],
            compressed: cmp_size as u32,
            header_size: size_of::<Self>() as u32
        }
    }
    pub fn to_bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(&raw const *self as _, size_of::<Self>()) }
    }
}

const _: () = {
    ["Size of file header"][size_of::<DataHeader>() - 0x30];
};