import Grid from "@material-ui/core/Grid";
import React from "react";
import { Button, InputAdornment } from "@material-ui/core";
import TextField from "@material-ui/core/TextField";
import { DENOM } from "../../pages/_app";
import { theme } from "../../lib/theme";
import { basicRawCoinValueValidation, makeBasicStyle, validateIdentityKey } from "../../common/helpers";


type DelegateFormProps = {
    onSubmit: (event: any) => void
}

export default function DelegateForm(props: DelegateFormProps) {
    const classes = makeBasicStyle(theme);

    const [validAmount, setValidAmount] = React.useState(true)
    const [validIdentity, setValidIdentity] = React.useState(true)
    // const [checkboxSet, setCheckboxSet] = React.useState(false)

    // const handleCheckboxToggle = () => {
    //     setCheckboxSet((prevSet) => !prevSet);
    // }

    const handleAmountChange = (event: any) => {
        let nonZeroAmount = event.target.value.length > 0
        if (nonZeroAmount) {
            // don't ask me about that. javascript works in mysterious ways
            // and this is apparently a good way of checking if string
            // is purely made of numeric characters
            let parsed = +event.target.value
            if (isNaN(parsed)) {
                setValidAmount(false)
            } else {
                setValidAmount(true)
            }
        }
    }

    const validateForm = (event: any): boolean => {
        let validIdentity = validateIdentityKey(event.target.identity.value);
        let validAmount = validateAmount(event.target.amount.value);

        setValidIdentity(validIdentity)
        setValidAmount(validAmount)

        return validIdentity && validAmount
    }

    const validateAmount = (rawAmount: string): boolean => {
        return basicRawCoinValueValidation(rawAmount)
    }

    const submitForm = (event: any) => {
        event.preventDefault()

        if (validateForm(event)) {
            return props.onSubmit(event)
        }
    }

    return (
        <form onSubmit={submitForm}>
            <Grid container spacing={3}>

                <Grid item xs={12}>
                    <TextField
                        required
                        id="identity"
                        name="identity"
                        label="Node identity"
                        error={!validIdentity}
                        helperText={validIdentity ? "" : "Please enter a valid identity like '824WyExLUWvLE2mpSHBatN4AoByuLzfnHFeHWiBYzg4z'"}
                        fullWidth
                    />
                </Grid>

                <Grid item xs={12} sm={6}>
                    <TextField
                        required
                        id="amount"
                        name="amount"
                        label="Amount to delegate"
                        error={!validAmount}
                        helperText={validAmount ? "" : "Please enter a valid amount"}
                        onChange={handleAmountChange}
                        fullWidth
                        InputProps={{
                            endAdornment:
                                <InputAdornment position="end">{DENOM}</InputAdornment>
                        }}
                    />
                </Grid>

                {/*<Grid item xs={12}>*/}
                {/*    <FormControlLabel*/}
                {/*        control={*/}
                {/*            <Checkbox*/}
                {/*                checked={checkboxSet}*/}
                {/*                onChange={handleCheckboxToggle}*/}

                {/*            />*/}
                {/*        }*/}
                {/*        label="checkbox text"*/}
                {/*    />*/}
                {/*</Grid>*/}

            </Grid>
            <div className={classes.buttons}>
                <Button
                    variant="contained"
                    color="primary"
                    type="submit"
                    className={classes.button}
                >
                    Delegate stake
                </Button>
            </div>
        </form>
    )
}