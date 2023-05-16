package net.nymtech.nyms5

class Socks5 {
    // Load the native library "libsocks5-c.so".
    init {
        System.loadLibrary("socks5_c")
    }

    fun runtest(): String? {
        return run("TEST")
    }

    // Native function implemented in Rust.
    private external fun run(input: String): String?
}