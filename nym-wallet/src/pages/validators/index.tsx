import React, { useContext, useEffect, useState } from 'react';
import { Alert, Box, Dialog } from '@mui/material';
import { NymCard } from '../../components';
import { ClientContext } from '../../context/main';
import { ValidatorSelector } from './ValidatorSelector';
import { Delegate as DelegateIcon } from '../../svg-icons';

export const ValidatorSettings = () => {

    const { mixnodeDetails, showValidatorSettings, getBondDetails, handleShowSettings } = useContext(ClientContext);

    useEffect(() => {
        getBondDetails();
    }, [showValidatorSettings]);

    return showValidatorSettings ? (
        <Dialog open onClose={handleShowSettings} maxWidth="md" fullWidth>
            <NymCard
                title={
                    <Box display="flex" alignItems="center">
                        <DelegateIcon sx={{ mr: 1 }} />
                        Settings
                    </Box>
                }
            >
                <ValidatorSelector
                    onChangeValidatorSelection={(selectedValidator) => console.log('selectedValidator:', selectedValidator)}
                />
            </NymCard>
        </Dialog>
    ) : null;
};
