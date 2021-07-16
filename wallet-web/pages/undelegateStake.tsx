import React from "react";
import MainNav from "../components/MainNav";
import NodeUndelegation from "../components/undelegate/NodeUndelegation";

const UndelegateStake = () => {
    return (
        <React.Fragment>
            <MainNav />
            <NodeUndelegation />
        </React.Fragment>
    );
}

export default UndelegateStake