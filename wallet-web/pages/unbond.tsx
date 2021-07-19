import React from "react";
import MainNav from "../components/MainNav";
import UnbondNode from "../components/unbond/UnbondNode";

const Unbond = () => {
    return (
        <React.Fragment>
            <MainNav/>
            <UnbondNode />
        </React.Fragment>
    );
}

export default Unbond