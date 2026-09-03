#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ByteType {
    Byte,
    HalfWord,
    Word,
    DoubleWord
}

impl ByteType {
    pub const fn as_num(&self) -> usize {
        match self {
            ByteType::Byte => 1,
            ByteType::HalfWord => 2,
            ByteType::Word => 4,
            ByteType::DoubleWord => 8,
        }
    }
}

pub fn as_byte_type(num: usize) -> Option<ByteType> {
    match num {
        1 => Some(ByteType::Byte),
        2 => Some(ByteType::HalfWord),
        4 => Some(ByteType::Word),
        8 => Some(ByteType::DoubleWord),
        _ => None,
    }
}