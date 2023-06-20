pub const LIB25519_DH_PUBLICKEYBYTES: usize = 32;
pub const LIB25519_DH_SECRETKEYBYTES: usize = 32;
pub const LIB25519_DH_BYTES: usize = 32;

pub const LIB25519_SIGN_SECRETKEYBYTES: usize = 64;
pub const LIB25519_SIGN_PUBLICKEYBYTES: usize = 32;
pub const LIB25519_SIGN_BYTES: usize = 64;

#[link(name = "25519", kind = "static")]
extern "C" {
    pub fn lib25519_dh_x25519_keypair(
        arg1: *mut ::std::os::raw::c_uchar,
        arg2: *mut ::std::os::raw::c_uchar,
    );
    pub fn lib25519_dh_x25519(
        arg1: *mut ::std::os::raw::c_uchar,
        arg2: *const ::std::os::raw::c_uchar,
        arg3: *const ::std::os::raw::c_uchar,
    );
    pub fn lib25519_sign_ed25519_keypair(
        arg1: *mut ::std::os::raw::c_uchar,
        arg2: *mut ::std::os::raw::c_uchar,
    );
    pub fn lib25519_sign_ed25519(
        arg1: *mut ::std::os::raw::c_uchar,
        arg2: *mut ::std::os::raw::c_longlong,
        arg3: *const ::std::os::raw::c_uchar,
        arg4: ::std::os::raw::c_longlong,
        arg5: *const ::std::os::raw::c_uchar,
    );
    pub fn lib25519_sign_ed25519_open(
        arg1: *mut ::std::os::raw::c_uchar,
        arg2: *mut ::std::os::raw::c_longlong,
        arg3: *const ::std::os::raw::c_uchar,
        arg4: ::std::os::raw::c_longlong,
        arg5: *const ::std::os::raw::c_uchar,
    ) -> ::std::os::raw::c_int;
}
