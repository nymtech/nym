import React from 'react';
import ValidatorClient from "@nymproject/nym-validator-client";

type ClientContext = {
    client: ValidatorClient,
    setClient: (client: ValidatorClient) => void
}

const defaultValue: ClientContext = {
    client: null,
    setClient: () => { },
}

export const ValidatorClientContext = React.createContext(defaultValue);

