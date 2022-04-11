import React, { useContext, useEffect, useState } from 'react';
import { Button, Box, Dialog, CircularProgress, Typography } from '@mui/material';
import { NymCard } from '../../components';
import { ClientContext } from '../../context/main';
import { ValidatorSelector } from './validatorSelector';
import { Delegate as DelegateIcon } from '../../svg-icons';
import { Console } from '../../utils/console';

const tabs = ['Validators'];

export const ValidatorSettingsModal = () => {
    const [isSubmitting, setIsSubmitting] = useState(false);
    const [data, setData] = useState({});

    const { showValidatorSettings, getBondDetails, handleShowValidatorSettings } = useContext(ClientContext);

    useEffect(() => {
        getBondDetails();
    }, [showValidatorSettings]);

    const onDataChanged = (selectedValidator?: string, selectedAPI?: string) => {
        if (selectedValidator) {
            setData(selectedValidator);
        };
        console.log('selectedValidator:', selectedValidator, 'network', selectedAPI);
    }

    const handleSubmit = (data: {}) => {
        console.log('data', data);
    }

    return showValidatorSettings ? (
        <Dialog open onClose={handleShowValidatorSettings} maxWidth="md" fullWidth>
            <NymCard
                title={
                    <Box display="flex" alignItems="center">
                        <DelegateIcon sx={{ mr: 1 }} />
                        Settings
                    </Box>
                }
                noPadding
            >
                <Box
                    sx={{
                        display: 'flex',
                        alignItems: 'center',
                        padding: 3,
                        borderTop: '1px solid',
                        borderColor: 'grey.300',
                    }}
                >
                    <Typography
                        variant="h6"
                        sx={{
                            fontWeight: 600,
                        }}>
                        Wallet Settings
                    </Typography>
                </Box>
                <Box
                    sx={{
                        display: 'flex',
                        alignItems: 'center',
                        p: 2,
                        pl: 3,
                        borderTop: '1px solid',
                        borderBottom: '1px solid',
                        borderColor: 'grey.300',
                        bgcolor: 'grey.200',
                    }}
                >
                    <Typography
                        variant="h6"
                        sx={{
                            fontWeight: 600,
                        }}>
                        Validators
                    </Typography>
                </Box>
                <>
                    <Box
                        sx={{
                            display: 'flex',
                            alignItems: 'center',
                            padding: 3,
                        }}
                    >
                        <ValidatorSelector
                            type="Validator API Url"
                            onChangeValidatorSelection={(selectedValidator) => onDataChanged(selectedValidator)}
                        />
                    </Box>
                </>
                <Box
                    sx={{
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'flex-end',
                        padding: 3,
                        bgcolor: 'grey.200',
                        borderTop: '1px solid',
                        borderColor: 'grey.300',
                    }}
                >
                    <Button
                        size="large"
                        variant="contained"
                        data-testid="validatorsSettings-button"
                        color="primary"
                        disableElevation
                        onClick={async () => {
                            setIsSubmitting(true);
                            try {
                                console.log('hello')
                            } catch (e) {
                                Console.error(e as string);
                            } finally {
                                setIsSubmitting(false);
                            }
                        }}
                        disabled={isSubmitting}
                        endIcon={isSubmitting && <CircularProgress size={20} />}
                    >
                        Save Changes
                    </Button>
                </Box>
            </NymCard>
        </Dialog>
    ) : null;
};
