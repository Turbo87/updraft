package aero.updraft.mobile

import android.database.Cursor
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.lang.reflect.Proxy

class DocumentDisplayNameTest {
    @Test
    fun `reads the document name and closes its cursor`() {
        var closed = false
        val cursor = Proxy.newProxyInstance(
            Cursor::class.java.classLoader, arrayOf(Cursor::class.java)
        ) { _, method, args ->
            when (method.name) {
                "getColumnIndex" -> { assertEquals("_display_name", args[0]); 2 }
                "moveToFirst" -> true
                "getString" -> { assertEquals(2, args[0]); "Local waypoints.cup" }
                "close" -> { closed = true; null }
                else -> error("Unexpected cursor call: ${method.name}")
            }
        } as Cursor
        assertEquals("Local waypoints.cup", documentDisplayName(cursor))
        assertTrue(closed)
    }

    @Test
    fun `returns no name when the provider returns no cursor`() {
        assertEquals(null, documentDisplayName(null))
    }
}
