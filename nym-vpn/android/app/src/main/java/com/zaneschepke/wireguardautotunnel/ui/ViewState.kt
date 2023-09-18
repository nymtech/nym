package net.nymtech.nymconnect.ui

data class ViewState(
    val showSnackbarMessage : Boolean = false,
    val snackbarMessage : String = "",
    val snackbarActionText : String = "",
    val onSnackbarActionClick : () -> Unit = {},
    val isLoading : Boolean = false
)
