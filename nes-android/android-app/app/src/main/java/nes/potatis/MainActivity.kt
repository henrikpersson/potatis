package nes.potatis

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.os.Parcelable
import android.util.Log
import android.widget.Button
import androidx.annotation.Keep
import androidx.appcompat.app.AppCompatActivity
import kotlin.concurrent.thread

object Rust {
    external fun init(rom: ByteArray, b: Bindings, p: PanicHandler): Long
    external fun tick(ptr: Long)
    external fun destroy(ptr: Long)
}

class PanicHandler {
    @Keep
    fun panic(s: String) {
        Log.e("nes", s)
    }
}

class Bindings(
    private val onRender: (ByteArray) -> Unit,
    private val buttons: Array<Button>
) {
    @Keep
    fun render(pixels: ByteArray) {
        onRender(pixels)
    }

    @Keep
    fun input(): Byte {
        var ret = 0
        for ((i, btn) in buttons.withIndex()) {
            if (btn.isPressed) {
                ret = (ret shl i) or (1 shl i)
            }
        }
        return ret.toByte()
    }
}

class MainActivity : AppCompatActivity() {
    companion object {
        init {
            System.loadLibrary("nes_android")
        }
    }

    sealed class Nes
    class Running(val ptr: Long) : Nes()
    object Destroyed : Nes()

    private lateinit var nesView: NesView
    private var nes: Nes = Destroyed

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)
        nesView = findViewById(R.id.nes_view)

        val buttons = arrayOf(
            findViewById<Button>(R.id.b),
            findViewById(R.id.a),
            findViewById(R.id.up),
            findViewById(R.id.down),
            findViewById(R.id.left),
            findViewById(R.id.right),
            findViewById(R.id.start),
            findViewById(R.id.select)
        )

        val uri = intent.getParcelableExtra<Parcelable>(Intent.EXTRA_STREAM) as Uri?
        val rom = if (intent.action == Intent.ACTION_SEND && uri != null) {
            contentResolver.openInputStream(uri)?.use {
                it.readBytes()
            } ?: ByteArray(0)
        } else {
            ByteArray(0)
        }

        thread {
            nes = Running(Rust.init(rom, Bindings(this::onRender, buttons), PanicHandler()))

            while (true) {
                when (val nes = nes) {
                    is Running -> Rust.tick(nes.ptr)
                    is Destroyed -> break
                }
            }
        }
    }

    override fun onDestroy() {
        (nes as? Running)?.let { 
            nes = Destroyed
            Rust.destroy(it.ptr)
        }
        super.onDestroy()
    }

    private fun onRender(pixels: ByteArray) {
        runOnUiThread {
            nesView.render(pixels)
        }
    }
}