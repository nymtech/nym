import React, { useState } from "react";
import MainNav from "../components/MainNav";
import Paper from "@material-ui/core/Paper";
import Typography from "@material-ui/core/Typography";
import { makeStyles } from "@material-ui/core/styles";
import Button from "@material-ui/core/Button";
import ValidatorClient from "@nymproject/nym-validator-client";
import { LinearProgress } from "@material-ui/core";
import { useRouter } from "next/router";

const useStyles = makeStyles((theme) => ({
    appBar: {
        position: 'relative',
    },
    layout: {
        width: 'auto',
        marginLeft: theme.spacing(2),
        marginRight: theme.spacing(2),
        [theme.breakpoints.up(600 + theme.spacing(2) * 2)]: {
            width: 600,
            marginLeft: 'auto',
            marginRight: 'auto',
        },
    },
    paper: {
        marginTop: theme.spacing(3),
        marginBottom: theme.spacing(3),
        padding: theme.spacing(2),
        [theme.breakpoints.up(600 + theme.spacing(3) * 2)]: {
            marginTop: theme.spacing(6),
            marginBottom: theme.spacing(6),
            padding: theme.spacing(3),
        },
    },
    buttons: {
        display: 'flex',
        justifyContent: 'center',
    },
    button: {
        marginTop: theme.spacing(3),
        marginLeft: theme.spacing(1),
    },
}));


type AccountDetailsProps = {
    mnemonic: String,
    address: String
}

function AccountDetails(props: AccountDetailsProps) {
    if (props.mnemonic === "" || props.address === "") {
        return <React.Fragment />
    }

    return (
        // Yeah, I probably should have done it properly with CSS but inserting `<br />` was way quicker to achieve
        // the same result
        <React.Fragment>
            <br />

            <Typography variant="h6" align="center">
                Mnemonic
            </Typography>
            <Typography variant="body1">
                {props.mnemonic}
            </Typography>
            <br />

            <Typography variant="h6" align="center">
                Account Address
            </Typography>
            <Typography variant="body1">
                {props.address}
            </Typography>
            <br />

            <Typography variant="body2">
                Save your mnemonic in a safe space as it's your only way to recover your account!
            </Typography>
        </React.Fragment>
    )
}

export default function CreateAccount() {
    const classes = useStyles();
    const router = useRouter()

    const [mnemonic, setMnemonic] = useState("");
    const [address, setAddress] = useState("")

    const [loading, setLoading] = useState(false)
    const [created, setCreated] = useState(false)

    const createAccount = () => {
        let mnemonic = ValidatorClient.randomMnemonic();
        setMnemonic(mnemonic)

        ValidatorClient.mnemonicToAddress(mnemonic, "punk").then((address) => {
            setAddress(address)
            setCreated(true)
        })
    }

    const handleBack = async (event) => {
        setLoading(true)
        event.preventDefault()
        await router.push("/")
        // no need to set loading to false as at this point the component is unmounted
    }

    return (
        <React.Fragment>
            <MainNav />
            <main className={classes.layout}>
                <Paper className={classes.paper}>
                    <Typography component="h1" variant="h4" align="center">
                        Create new account
                    </Typography>
                    <AccountDetails mnemonic={mnemonic} address={address} />
                    {loading && <LinearProgress />}
                    <div className={classes.buttons}>
                        {!created ? (
                            <Button
                                variant="contained"
                                color="primary"
                                onClick={createAccount}
                                className={classes.button}
                            >
                                Create new
                            </Button>
                        ) : (
                            <Button
                                variant="contained"
                                color="primary"
                                onClick={handleBack}
                                className={classes.button}
                            >
                                Go back to sign in
                            </Button>
                        )
                        }
                    </div>
                </Paper>
            </main>
        </React.Fragment >
    )
}
