import React, { useContext, useState } from 'react';
import Button from '@material-ui/core/Button';
import CssBaseline from '@material-ui/core/CssBaseline';
import TextField from '@material-ui/core/TextField';
import Grid from '@material-ui/core/Grid';
import Typography from '@material-ui/core/Typography';
import { makeStyles } from '@material-ui/core/styles';
import Container from '@material-ui/core/Container';
import { ValidatorClientContext } from "../contexts/ValidatorClient";
import { useRouter } from "next/router";
import ValidatorClient from "@nymproject/nym-validator-client";
import { BONDING_CONTRACT_ADDRESS, DENOM, VALIDATOR_URLS } from "../pages/_app";
import { LinearProgress } from "@material-ui/core";
import { Alert, AlertTitle } from "@material-ui/lab";
import Link from "./Link";


const useStyles = makeStyles((theme) => ({
    paper: {
        marginTop: theme.spacing(8),
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
    },
    avatar: {
        margin: theme.spacing(1),
        backgroundColor: theme.palette.secondary.main,
    },
    form: {
        width: '100%', // Fix IE 11 issue.
        marginTop: theme.spacing(1),
    },
    submit: {
        margin: theme.spacing(3, 0, 2),
    },
}));

export default function SignIn() {
    const classes = useStyles();
    const router = useRouter()

    const { client, setClient } = useContext(ValidatorClientContext)
    const [loading, setLoading] = useState(false)
    const [clientError, setClientError] = useState(null)

    console.log("context client is", client);

    const makeClient = (mnemonic: string): Promise<boolean> => {
        return ValidatorClient.connect(
            BONDING_CONTRACT_ADDRESS,
            mnemonic,
            VALIDATOR_URLS,
            DENOM,
        ).then((client) => {
            setClient(client);
            console.log(`connected to validator, our address is ${client.address}`);
            console.log("connected to validator", client.urls[0])
            return true
        }).catch((err) => {
            setClientError(err)
            throw new Error("failed to create the client");
        });

    }

    const failedClient = (err: Error) => {
        return (
            <Alert severity="error">
                <AlertTitle>Could not create the client</AlertTitle>
                {err.message}
            </Alert>
        )
    }

    const handleSubmit = async (event) => {
        event.preventDefault()
        setLoading(true)
        setClientError(null)
        let mnemonic = event.target.mnemonic.value
        makeClient(mnemonic).then(async () => {
            // only push `/send` if we managed to create the client!
            await router.push("/bond")
        }
        ).catch((_err) => {
            setLoading(false)
        })
    }

    return (
        <Container component="main" maxWidth="xs">
            <CssBaseline />
            <div className={classes.paper}>
                {/* <Avatar className={classes.avatar}>
                    <LockOutlinedIcon />
                </Avatar> */}
                <Typography component="h1" variant="h5">
                    Sign in
                </Typography>
                <form className={classes.form} noValidate onSubmit={handleSubmit}>
                    <TextField
                        variant="outlined"
                        margin="normal"
                        required
                        fullWidth
                        id="mnemonic"
                        label="BIP-39 Mnemonic"
                        name="mnemonic"
                        autoComplete="mnemonic"
                        autoFocus
                    />
                    {clientError !== null && failedClient(clientError)}

                    {loading && <LinearProgress />}
                    <Button
                        fullWidth
                        variant="contained"
                        color="primary"
                        type="submit"
                        className={classes.submit}
                        disabled={loading}
                    >
                        Sign In
                    </Button>
                    <Grid container>
                        <Grid item>
                            <Link href="/createAccount" variant="body2">
                                {"Don't have an account? Create one"}
                            </Link>
                        </Grid>
                    </Grid>
                </form>
            </div>
        </Container>
    );
}
