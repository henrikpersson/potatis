package nes.potatis

import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.util.AttributeSet
import android.view.View

class NesView(context: Context, attrs: AttributeSet?) : View(context, attrs) {
    private val pixelSize = resources.displayMetrics.density
    private val paint = Paint().apply { style = Paint.Style.FILL }
    private var pixels: ByteArray? = null

    fun render(pixels: ByteArray) {
        this.pixels = pixels
        invalidate()
    }

    override fun onDraw(canvas: Canvas) {
        val pixels = this.pixels ?: return

        var x = 0f
        var y = 0f
        pixels.iterator()
            .asSequence()
            .map { it.toUByte().toInt() }
            .chunked(3)
            .forEach {
                paint.color = Color.rgb(it[0], it[1], it[2])
                canvas.drawRect(x, y, x + pixelSize, y + pixelSize, paint)

                x += pixelSize

                if (x == canvas.width.toFloat()) {
                    x = 0f
                    y += pixelSize
                }
            }
    }
}