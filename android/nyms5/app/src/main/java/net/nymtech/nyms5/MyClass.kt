package net.nymtech.nyms5

class MyClass {
    // Load the native library "libsocks5_c.so".
    init {
        System.loadLibrary("socks5_c")
    }

    fun run(): String? {
        return runclient()
    }

    // Native function implemented in Rust.
    private external fun runclient(): String?
}