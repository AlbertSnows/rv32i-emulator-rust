use std::collections::VecDeque;

// matches QEMU's virt machine convention (UART0_IRQ = 10)
pub const UART_SOURCE_ID: usize = 10;

#[derive(Clone, Debug, PartialEq)]
pub struct UartState {
    pub(crate) rx_buffer: VecDeque<u8>
}

impl UartState {

    pub fn receive_byte(&mut self, byte: u8) {
        self.rx_buffer.push_back(byte);
    }

    pub fn read(&mut self, offset: u32, num_bytes: usize) -> u32 {
        if offset == 5 {
            0x60
        } else if offset == 0 {
            self.rx_buffer.pop_front().unwrap_or(0) as u32
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