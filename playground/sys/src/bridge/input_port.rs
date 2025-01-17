#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("playground-sys/src/ref/bridge/input_port.hpp");

        type register_set_ptr = crate::bridge::register_set::register_set_ptr;

        type input_port_t;
        type input_port_bridge;

        #[must_use]
        unsafe fn new_input_port_bridge(ptr: *const input_port_t) -> SharedPtr<input_port_bridge>;
        #[must_use]
        fn inner(self: &input_port_bridge) -> *const input_port_t;
        #[must_use]
        fn get_cu_sets(self: &input_port_bridge) -> &CxxVector<u32>;
        #[must_use]
        fn get_in_ports(self: &input_port_bridge) -> UniquePtr<CxxVector<register_set_ptr>>;
        #[must_use]
        fn get_out_ports(self: &input_port_bridge) -> UniquePtr<CxxVector<register_set_ptr>>;
    }

    // explicit instantiation for input_port_t to implement VecElement
    impl CxxVector<input_port_t> {}
}

pub use ffi::*;
