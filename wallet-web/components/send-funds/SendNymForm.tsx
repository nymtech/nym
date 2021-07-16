import React from 'react';
import Grid from '@material-ui/core/Grid';
import Typography from '@material-ui/core/Typography';
import TextField from '@material-ui/core/TextField';
import { DENOM } from '../../pages/_app';
import { InputAdornment } from "@material-ui/core";

type SendNymFormProps = {
    address: string,
    setFormStatus: (nonEmpty: boolean) => void,
}

export default function SendNymForm({ address, setFormStatus }: SendNymFormProps) {
    const [recipientHasValue, setRecipientHasValue] = React.useState(false)
    const [amountHasValue, setAmountHasValue] = React.useState(false)
    const [validAmount, setValidAmount] = React.useState(true)

    const handleInputData = (element) => (event) => {
        if (element === "recipient") {
            let nonZeroRecipient = event.target.value.length > 0
            setRecipientHasValue(nonZeroRecipient)
            setFormStatus(nonZeroRecipient && amountHasValue)
        } else if (element === "amount") {
            let nonZeroAmount = event.target.value.length > 0
            setAmountHasValue(nonZeroAmount)
            setFormStatus(recipientHasValue && nonZeroAmount)
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

    }

    return (
        <React.Fragment>
            <Typography variant="h6" gutterBottom>
                Enter recipient address and the amount
            </Typography>
            <Grid container spacing={3}>
                <Grid item xs={12}>
                    <Typography variant="h6">Sending from {address}</Typography>
                </Grid>
                <Grid item xs={12}>
                    <TextField
                        required
                        id="recipient"
                        name="recipient"
                        label="Recipient address"
                        onChange={handleInputData("recipient")}
                        fullWidth
                    />
                </Grid>
                <Grid item xs={12} sm={6}>
                    <TextField
                        required
                        id="amount"
                        name="amount"
                        label="Amount"
                        error={!validAmount}
                        helperText={validAmount ? "" : "Please enter a valid amount"}
                        onChange={handleInputData("amount")}
                        fullWidth
                        InputProps={{
                            endAdornment:
                            <InputAdornment position="end">{DENOM}</InputAdornment>
                        }}
                    />
                </Grid>
            </Grid>
        </React.Fragment>
    );
}
