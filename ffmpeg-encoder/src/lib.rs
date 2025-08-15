unsafe extern "C" {
    fn hello_world();
}

pub fn test() {
    unsafe {
        hello_world();
    }
}
