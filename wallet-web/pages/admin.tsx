import { useRouter } from "next/router";
import React, { useContext, useEffect, useState } from "react";
import { ValidatorClientContext } from "../contexts/ValidatorClient";
import MainNav from "../components/MainNav";
import Paper from "@material-ui/core/Paper";
import Typography from "@material-ui/core/Typography";
import NoClientError from "../components/NoClientError";
import { makeStyles } from "@material-ui/core/styles";
import { ADMIN_ADDRESS } from "./_app";
import { LinearProgress } from "@material-ui/core";
import AdminForm from "../components/admin/AdminForm";
import { StateParams } from "@nymproject/nym-validator-client";
import { Alert, AlertTitle } from "@material-ui/lab";
import { printableBalanceToNative } from "@nymproject/nym-validator-client/dist/currency";

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
        justifyContent: 'flex-end',
    },
    button: {
        marginTop: theme.spacing(3),
        marginLeft: theme.spacing(1),
    },
}));

export default function Admin() {
    const classes = useStyles();
    const router = useRouter()

    const {client} = useContext(ValidatorClientContext)

    const [currentStateParams, setCurrentStateParams] = useState(null)
    const [updateError, setUpdateError] = useState(null)
    const [updatingState, setUpdatingState] = useState(false)

    // naive way of preventing non-tech-savvy people from accessing this page by manually changing
    // page to /admin and thinking they're "l33t h4x0rs".
    // However, even if somebody does access this page by telling the page their address is
    // the same as that of the current admin,
    // nothing is going to happen as they will be unable to sign a transaction to the contract without having
    // access to the account itself
    useEffect(() => {
        (async () => {
            if (client === null || client.address !== ADMIN_ADDRESS) {
                await router.push("404")
            } else {
                let params = await client.getStateParams()
                setCurrentStateParams(params)
            }
        })()
    }, [client])

    const updateStateParams = async (event) => {
        event.preventDefault()
        let newState: StateParams = {
            // need to convert those to native coin (i.e. hal -> uhal)
            minimum_mixnode_bond: printableBalanceToNative(event.target.mix_bond.value),
            minimum_gateway_bond: printableBalanceToNative(event.target.gateway_bond.value),
            mixnode_bond_reward_rate: event.target.mix_reward.value,
            gateway_bond_reward_rate: event.target.gateway_reward.value,
            epoch_length: parseInt(event.target.epoch_length.value),
            mixnode_active_set_size: parseInt(event.target.active_set.value),
        };
        setUpdatingState(true)
        await client.updateStateParams(newState)
            .then((_) => setCurrentStateParams(newState))
            .catch((err) => setUpdateError(err))
            .finally(() => setUpdatingState(false))
    }

    const getAdminContent = () => {
        // we're not signed in (I guess it should get displayed super briefly before getting forwarded
        // to a 404)
        if (client === null) {
            return (<NoClientError/>)
        }

        // we encountered error while trying to update state
        if (updateError !== null) {
            return (
                <Alert severity="error">
                    <AlertTitle>{updateError.name}</AlertTitle>
                    <strong>Failed to update contract state</strong> - {updateError.message}
                </Alert>
            )
        }

        // We're getting current state params
        if (currentStateParams === null) {
            return (<LinearProgress/>)
        }

        if (updatingState) {
            return (
                <React.Fragment>
                    <br />
                    <Typography component="h1" variant="h5" align="center">
                        Updating contract state...
                    </Typography>
                    <LinearProgress/>
                </React.Fragment>
            )
        }

        return (<AdminForm onSubmit={updateStateParams} currentParams={currentStateParams}/>)
    }

    return (
        <React.Fragment>
            <MainNav/>
            <main className={classes.layout}>
                <Paper className={classes.paper}>
                    <Typography component="h1" variant="h4" align="center">
                        Contract control
                    </Typography>

                    {getAdminContent()}
                </Paper>
            </main>
        </React.Fragment>
    )
}