package net.nymtech.nyms5

import android.util.Log
import io.sentry.Sentry

const val nymNativeLib = "nym_socks5_listener"

class NymProxy {
    private val tag = "NymProxy"

    companion object {
        enum class State {
            UNINITIALIZED,
            CONNECTED,
            DISCONNECTED
        }
    }

    // Load the native library "libnym_socks5_listener.so"
    init {
        System.loadLibrary(nymNativeLib)
        Log.i(tag, "loaded native library $nymNativeLib")
    }

    // this is a blocking call as `startClient` is blocking and will releases when
    // the socks5 connection has been terminated
    fun start(serviceProvider: String, onStartCbObj: Any, onStopCbObj: Any) {
        Log.d(tag, "calling $nymNativeLib:startClient")
        try {
            startClient(serviceProvider, onStartCbObj, onStopCbObj)
        } catch (e: Throwable) {
            Log.e(tag, "$nymNativeLib:startClient internal error: $e")
            Sentry.captureException(e)
        }
    }

    fun stop() {
        Log.d(tag, "calling $nymNativeLib:stopClient")
        try {
            stopClient()
        } catch (e: Throwable) {
            Log.e(tag, "$nymNativeLib:stopClient internal error: $e")
            Sentry.captureException(e)
        }
    }

    fun ping(): Boolean {
        Log.d(tag, "calling $nymNativeLib:pingClient")
        try {
            return pingClient()
        } catch (e: Throwable) {
            Log.e(tag, "$nymNativeLib:pingClient internal error: $e")
            Sentry.captureException(e)
        }
        return false
    }

    fun getState(): State {
        Log.d(tag, "calling $nymNativeLib:getClientState")
        try {
            return when (getClientState()) {
                0 -> State.UNINITIALIZED
                1 -> State.CONNECTED
                2 -> State.DISCONNECTED
                else -> throw Error("unknown state")
            }
        } catch (e: Throwable) {
            Log.e(tag, "$nymNativeLib:getClientState internal error: $e")
            Sentry.captureException(e)
        }
        return State.UNINITIALIZED
    }

    private external fun startClient(
        spAddress: String,
        onStartCbObj: Any,
        onStopCbObj: Any
    )

    private external fun stopClient()
    private external fun getClientState(): Int
    private external fun pingClient(): Boolean
}