use std::io::Read;

pub trait FromLeBytes<const N: usize> {
    fn from_bytes(bytes: [u8; N]) -> Self;
}
impl FromLeBytes<4> for u32 {
    fn from_bytes(bytes: [u8; 4]) -> Self {
        u32::from_le_bytes(bytes)
    }
}
impl FromLeBytes<4> for f32 {
    fn from_bytes(bytes: [u8; 4]) -> Self {
        f32::from_le_bytes(bytes)
    }
}
impl FromLeBytes<2> for u16 {
    fn from_bytes(bytes: [u8; 2]) -> Self {
        u16::from_le_bytes(bytes)
    }
}
impl FromLeBytes<2> for i16 {
    fn from_bytes(bytes: [u8; 2]) -> Self {
        i16::from_le_bytes(bytes)
    }
}
pub struct Reader<T>
where
    T: Read,
{
    reader: T,
}

impl<T> Reader<T>
where
    T: Read,
{
    pub fn new(reader: T) -> Self
    where
        T: Read,
    {
        Self { reader }
    }
    pub fn read(&mut self, len: usize) -> Vec<u8> {
        let mut vec = vec![0u8; len];
        self.reader.read_exact(vec.as_mut_slice()).unwrap();
        vec
    }
    // fn check_header(&mut self, header: &[u8]) -> bool {
    //     return self.read(header.len()) == header;
    // }
    pub fn check_header(&mut self, header: &str) -> bool {
        self.read(header.len()) == header.as_bytes()
    }
    pub fn read_type<I, const N: usize>(&mut self) -> I
    where
        I: FromLeBytes<N>,
    {
        let mut buf = [0u8; N];
        self.reader.read_exact(&mut buf).unwrap();
        <I>::from_bytes(buf)
    }
}
