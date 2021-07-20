import React, { useContext, useEffect } from "react";
import { Paper } from "@material-ui/core";
import Typography from "@material-ui/core/Typography";
import { useRouter } from "next/router";
import { ValidatorClientContext } from "../../contexts/ValidatorClient";
import { NodeType } from "../../common/node";
import NoClientError from "../NoClientError";
import Confirmation from "../Confirmation";
import { theme } from "../../lib/theme";
import { makeBasicStyle } from "../../common/helpers";
import NodeTypeChooser from "../NodeTypeChooser";
import NodeIdentityForm from "../NodeIdentityForm";
import ExecFeeNotice from "../ExecFeeNotice";


const UndelegateFromNode = () => {
    const classes = makeBasicStyle(theme);
    const router = useRouter()
    const { client } = useContext(ValidatorClientContext)

    const [undelegationStarted, setUndelegationStarted] = React.useState(false)
    const [undelegationFinished, setUndelegationFinished] = React.useState(false)
    const [undelegationError, setUndelegationError] = React.useState(null)

    const [nodeType, setNodeType] = React.useState(NodeType.Mixnode)

    useEffect(() => {
        const checkClient = async () => {
            if (client === null) {
                await router.push("/")
            }
        }
        checkClient()
    }, [client])


    const undelegateFromNode = async (event) => {
        event.preventDefault();

        console.log(`UNDELEGATE button pressed`);

        let address = event.target.identity.value
        setUndelegationStarted(true)

        if (nodeType == NodeType.Mixnode) {
            client.removeMixnodeDelegation(address).then((value => {
                console.log("undelegated from mixnode!", value)
            })).catch(setUndelegationError).finally(() => setUndelegationFinished(true))
        } else {
            client.removeGatewayDelegation(address).then((value => {
                console.log("undelegated from gateway!", value)
            })).catch(setUndelegationError).finally(() => setUndelegationFinished(true))
        }
    }


    const getUndelegationContent = () => {
        // we're not signed in
        if (client === null) {
            return (<NoClientError />)
        }

        // we haven't clicked undelegate button yet
        if (!undelegationStarted) {
            return (
                <React.Fragment >
                    <NodeTypeChooser nodeType={nodeType} setNodeType={setNodeType} />
                    <NodeIdentityForm onSubmit={undelegateFromNode}  buttonText={"Remove delegation"}/>
                </React.Fragment>
            )
        }

        // We started delegation
        return (
            <Confirmation
                finished={undelegationFinished}
                error={undelegationError}
                progressMessage={`${nodeType} undelegation is in progress...`}
                successMessage={`${nodeType} undelegation was successful!`}
                failureMessage={`Failed to undelegate from a ${nodeType}!`}
            />
        )
    }

    return (
        <React.Fragment>
            <main className={classes.layout}>
                <Paper className={classes.paper}>
                    <ExecFeeNotice name={"undelegating stake"}/>
                    <Typography component="h1" variant="h4" align="center" className={classes.wrapper}>
                        Undelegate stake from {nodeType}
                    </Typography>
                    {getUndelegationContent()}
                </Paper>
            </main>
        </React.Fragment>
    );
}


export default UndelegateFromNode;
