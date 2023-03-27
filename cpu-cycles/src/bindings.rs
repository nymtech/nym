#[link(name = "cpucycles", kind = "static")]
extern "C" {
    pub static mut cpucycles:
        ::std::option::Option<unsafe extern "C" fn() -> ::std::os::raw::c_longlong>;
    pub fn cpucycles_implementation() -> *const ::std::os::raw::c_char;
    pub fn cpucycles_version() -> *const ::std::os::raw::c_char;
    pub fn cpucycles_persecond() -> ::std::os::raw::c_longlong;
    pub fn cpucycles_tracesetup();
}
