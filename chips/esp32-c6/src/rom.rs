//! Bindings to ROM routines

// todo: might want to be careful about relying on ROM routines

extern "C" {
    fn uart_tx_one_char(c: u8) -> usize;
}

pub fn write_bytes(bytes: &[u8]) {
    for b in bytes {
        unsafe {
            uart_tx_one_char(*b);
        }
    }
}

pub fn println(s: &str) {
    write_bytes(s.as_bytes());
    write_bytes(b"\r\n");
}
