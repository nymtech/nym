package net.nymtech.nymconnect.ui.common

import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.unit.dp

@OptIn(ExperimentalFoundationApi::class)
@Composable
fun RowListItem(leadingIcon : ImageVector? = null, leadingIconColor : Color = Color.Gray, text : String, onHold : () -> Unit, onClick: () -> Unit, rowButton : @Composable() () -> Unit ) {
    Box(
        modifier = Modifier
            .combinedClickable(
                onClick = {
                    onClick()
                },
                onLongClick = {
                    onHold()
                }
            )
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(14.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            Row(verticalAlignment = Alignment.CenterVertically,) {
                if(leadingIcon != null) {
                    Icon(
                        leadingIcon, "status",
                        tint = leadingIconColor,
                        modifier =  Modifier.padding(end = 10.dp).size(15.dp)
                    )
                }
                Text(text)
            }

            rowButton()
        }
    }
}