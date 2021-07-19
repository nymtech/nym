import React from "react";
import MainNav from "../components/MainNav";
import DelegationCheck from "../components/delegation-check/DelegationCheck";

const CheckDelegation = () => {
    return (
        <React.Fragment>
            <MainNav />
            <DelegationCheck />
        </React.Fragment>
    );
}

export default CheckDelegation