package net.nymtech.nyms5

class Socsk5 {
    // Load the native library "libsimd.so".
    init {
        System.loadLibrary("socks5_c")
    }

    fun run(): String? {
        return runclient()
    }

    // Native function implemented in Rust.
    private external fun runclient(): String?
}