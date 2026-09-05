package aero.updraft.mobile

import android.database.Cursor
import android.provider.OpenableColumns

internal fun documentDisplayName(cursor: Cursor?): String? = cursor?.use {
    val column = it.getColumnIndex(OpenableColumns.DISPLAY_NAME)
    if (column < 0 || !it.moveToFirst()) null else it.getString(column)
}
