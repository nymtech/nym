import React, { useContext, useEffect, useState } from 'react';
import { Button, Box, Dialog, CircularProgress, Typography } from '@mui/material';
import { NymCard } from '../../components';
import { ClientContext } from '../../context/main';
import { ValidatorSelector } from './validatorSelector';
import { Delegate as DelegateIcon } from '../../svg-icons';
import { Console } from '../../utils/console';

export const ValidatorSettingsModal = () => {
    const [isSubmitting, setIsSubmitting] = useState(false);
    const [validatorSelectedSuccessfully, setValidatorSelectedSuccessfully] = useState(false);
    const [validator, setValidator] = useState('');

    const { showValidatorSettings, getBondDetails, handleShowValidatorSettings, network, selectValidatorNymd } = useContext(ClientContext);


    useEffect(() => {
        if (showValidatorSettings) {
            getBondDetails();
        } else {
            setValidatorSelectedSuccessfully(false);
        };
    }, [showValidatorSettings]);

    const onDataChanged = (selectedValidator: string) => {
        if (selectedValidator) {
            setValidator(selectedValidator);
        };
    }

    const handleSubmit = async () => {
        setIsSubmitting(true);
        try {
            if (network) {
                selectValidatorNymd(validator, network).then(res => console.log('res', res));
                setValidatorSelectedSuccessfully(true);
            }
        } catch (e) {
            Console.error(e as string);
        } finally {
            setIsSubmitting(false);
        }
    };

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
                            flexDirection: 'column',
                            alignItems: 'start',
                            minHeight: 300,
                            padding: 3,
                        }}
                    >
                        <ValidatorSelector
                            type="Validator API Url"
                            onChangeValidatorSelection={(selectedValidator) => onDataChanged(selectedValidator)}
                        />

                        {validatorSelectedSuccessfully && (
                            <Typography sx={{ pt: 2, fontSize: 12, color: (theme) => theme.palette.success.light }}>
                                Successfully selected the validator: {validator}
                            </Typography>
                        )}
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
                        onClick={() => handleSubmit()}
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
