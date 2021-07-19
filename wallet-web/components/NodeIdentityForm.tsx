import TextField from "@material-ui/core/TextField";
import { Button } from "@material-ui/core";
import React from "react";
import { makeBasicStyle, validateIdentityKey } from "../common/helpers";
import { theme } from "../lib/theme";

type NodeIdentityFormProps = {
    onSubmit: (event: any) => void
    buttonText: string
}

const NodeIdentityForm = (props: NodeIdentityFormProps) => {
    const classes = makeBasicStyle(theme);

    const [validIdentity, setValidIdentity] = React.useState(true)

    const validateForm = (event: any): boolean => {
        let validIdentity = validateIdentityKey(event.target.identity.value);
        setValidIdentity(validIdentity)

        return validIdentity
    }

    const submitForm = (event: any) => {
        event.preventDefault()

        if (validateForm(event)) {
            return props.onSubmit(event)
        }
    }
    return (
        <form onSubmit={submitForm}>
            <TextField
                required
                id="identity"
                name="identity"
                label="Node identity"
                error={!validIdentity}
                helperText={validIdentity ? "" : "Please enter a valid identity like '824WyExLUWvLE2mpSHBatN4AoByuLzfnHFeHWiBYzg4z'"}
                fullWidth
            />
            <div className={classes.buttons}>
                <Button
                    variant="contained"
                    color="primary"
                    type="submit"
                    className={classes.button}
                >
                    {props.buttonText}
                </Button>
            </div>
        </form>
    )
}

export default NodeIdentityForm