unsafe extern "C" {
    fn start_encoder();
}

pub fn test() {
    unsafe {
        start_encoder();
    }
}
