import React, { useContext, useEffect, useState } from 'react';
import { Button, Box, Dialog, CircularProgress, Typography } from '@mui/material';
import { Tabs } from '../settings/tabs';
import { NymCard } from '../../components';
import { ClientContext } from '../../context/main';
import { ValidatorSelector } from './validatorSelector';
import { Delegate as DelegateIcon } from '../../svg-icons';
import { Console } from '../../utils/console';

const tabs = ['Validators', 'APIs'];

export const ValidatorSettingsModal = () => {
    const [selectedTab, setSelectedTab] = useState(0);
    const [isSubmitting, setIsSubmitting] = useState(false);
    const [data, setData] = useState({});

    const { showValidatorSettings, getBondDetails, handleShowValidatorSettings } = useContext(ClientContext);

    const handleTabChange = (_: React.SyntheticEvent, newTab: number) => setSelectedTab(newTab);

    useEffect(() => {
        getBondDetails();
    }, [showValidatorSettings]);

    const onDataChanged = (selectedValidator?: string, selectedAPI?: string) => {
        if (selectedValidator) {
            setData(selectedValidator);
        };
        if (selectedAPI) {
            setData(selectedAPI);
        };
        console.log('selectedValidator:', selectedValidator, 'selectedAPI', selectedAPI);
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
                <>
                    <Tabs tabs={tabs} selectedTab={selectedTab} onChange={handleTabChange} disabled={false} />
                    <Box
                        sx={{
                            display: 'flex',
                            alignItems: 'center',
                            padding: 3,
                        }}
                    >
                        {selectedTab === 0 &&
                            <ValidatorSelector
                                type="Validator API Url"
                                onChangeValidatorSelection={(selectedValidator) => onDataChanged(selectedValidator)}
                            />
                        }
                        {selectedTab === 1 &&
                            <ValidatorSelector
                                type={tabs[selectedTab]} onChangeValidatorSelection={(selectedAPI) => onDataChanged(selectedAPI)}
                            />
                        }
                    </Box>
                </>
                <Box
                    sx={{
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'flex-end',
                        padding: 3,
                        bgcolor: 'grey.200',
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
