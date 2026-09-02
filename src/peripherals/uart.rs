#[derive(Clone, Debug, PartialEq)]
pub struct UartState {
    pub(crate) rx_buffer: Option<u8>
}

impl UartState {
    pub fn read(&self, offset: u32, num_bytes: usize) -> u32 {
        if offset == 5 {
            0x60
        } else {
            0
        }
    }

    pub fn write(&self, offset: u32, bytes: &[u8]) {
        if offset == 0 {
            print!("{}", bytes[0] as char)
        }
    }
}