import {Alert, AlertTitle} from "@material-ui/lab";
import React from "react";
import Link from "./Link";

export default function NoClientError () {
    return (
        <Alert severity="error">
            <AlertTitle>No client detected</AlertTitle>
            Have you signed in? Try to go back to <Link href = "/">the main page</Link> and try again
        </Alert>
    )
}