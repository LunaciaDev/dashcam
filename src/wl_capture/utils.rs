/// Helper macro to unwrap Option<T> from object registry
/// [TODO]: Add an enum and error handling instead of panic
macro_rules! get_wl_object {
    ($x:expr) => {
        match $x.as_ref() {
            Some(t) => t,
            None => panic!(),
        }
    };
}

pub(crate) use get_wl_object;