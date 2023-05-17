package net.nymtech.nyms5

class Socks5 {
    // Load the native library "libsocks5-c.so".
    init {
        System.loadLibrary("socks5_c")
    }

    fun runClient(): String {
        return run("TEST")
    }

    fun start() {
        return startClient()
    }

    fun stop() {
        return stopClient()
    }

    private external fun startClient()
    private external fun stopClient()
    private external fun run(arg: String): String
}