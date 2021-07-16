import React, { useContext, useEffect } from "react";
import { Button, Paper } from "@material-ui/core";
import Typography from "@material-ui/core/Typography";
import { useRouter } from "next/router";
import { ValidatorClientContext } from "../../contexts/ValidatorClient";
import { NodeType } from "../../common/node";
import NoClientError from "../NoClientError";
import Confirmation from "../Confirmation";
import NodeTypeChooser from "../NodeTypeChooser";
import { printableCoin } from "@nymproject/nym-validator-client";
import { theme } from "../../lib/theme";
import { makeBasicStyle } from "../../common/helpers";
import NodeIdentityForm from "../NodeIdentityForm";


const DelegationCheck = () => {
    const classes = makeBasicStyle(theme);
    const router = useRouter()
    const {client} = useContext(ValidatorClientContext)

    const [checkStarted, setCheckStarted] = React.useState(false)
    const [checkFinished, setCheckFinished] = React.useState(false)
    const [checkError, setCheckError] = React.useState(null)

    const [nodeType, setNodeType] = React.useState(NodeType.Mixnode)
    const [stakeValue, setStakeValue] = React.useState("0")
    const [nodeIdentity, setNodeIdentity] = React.useState("")


    useEffect(() => {
        const checkClient = async () => {
            if (client === null) {
                await router.push("/")
            }
        }
        checkClient()
    }, [client])


    // eh, crude, but I guess does the trick
    const handleDelegationCheckError = (err: Error) => {
        if (err.message.includes("Could not find any delegation information associated with")) {
            setStakeValue("0 HAL")
        } else {
            setCheckError(err)
        }
    }

    const checkDelegation = async (event) => {
        event.preventDefault();

        console.log(`CHECK DELEGATION button pressed`);

        let identity = event.target.identity.value
        setNodeIdentity(identity)
        setCheckStarted(true)

        if (nodeType == NodeType.Mixnode) {
            client.getMixDelegation(identity, client.address).then((value => {
                setStakeValue(printableCoin(value.amount))
            })).catch(handleDelegationCheckError).finally(() => setCheckFinished(true))
        } else {
            client.getGatewayDelegation(identity, client.address).then((value => {
                setStakeValue(printableCoin(value.amount))
            })).catch(handleDelegationCheckError).finally(() => setCheckFinished(true))
        }

    }

    const getDelegationCheckContent = () => {
        // we're not signed in
        if (client === null) {
            return (<NoClientError/>)
        }

        // we haven't clicked delegate button yet
        if (!checkStarted) {
            return (
                <React.Fragment>
                    <NodeTypeChooser nodeType={nodeType} setNodeType={setNodeType}/>
                    <NodeIdentityForm onSubmit={checkDelegation} buttonText="Check stake value"/>
                </React.Fragment>
            )
        }

        // We started the check
        const stakeMessage = `Current stake on ${nodeType} ${nodeIdentity} is ${stakeValue}`
        return (
            <Confirmation
                finished={checkFinished}
                error={checkError}
                progressMessage={`${nodeType} (${nodeIdentity}) stake check is in progress...`}
                successMessage={stakeMessage}
                failureMessage={`Failed to check stake value on ${nodeType} ${nodeIdentity}!`}
            />
        )
    }

    return (
        <React.Fragment>
            <main className={classes.layout}>
                <Paper className={classes.paper}>
                    <Typography component="h1" variant="h4" align="center" className={classes.wrapper}>
                        Check your stake on a {nodeType}
                    </Typography>
                    {getDelegationCheckContent()}
                </Paper>
            </main>
        </React.Fragment>
    );
}


export default DelegationCheck;
