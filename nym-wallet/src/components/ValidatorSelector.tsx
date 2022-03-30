import React, { useContext, useEffect, useState } from 'react';
import { ListItemText, MenuItem, Select, SelectChangeEvent, Typography } from '@mui/material';
import { ClientContext } from '../context/main';
import { validatorUrls } from '../utils';

type TValidatorUrl = string;

export const ValidatorSelector: React.FC<{ onChangeValidatorSelection: (validator: TValidatorUrl) => void }> = ({
    onChangeValidatorSelection,
  }) => {
    const [validators, setValidators] = useState<string[] | null>();
    const [selectedValidator, setSelectedValidator] = useState<TValidatorUrl>('');

    const {
        network
    } = useContext(ClientContext);

    useEffect(() => {
        (async () => {
            if(network) {
            const validator = await validatorUrls(network);
            setValidators(validator?.urls);
            }
        })();
    }, []);
  
    useEffect(() => {
        onChangeValidatorSelection(selectedValidator);
    }, [selectedValidator]);

    return (
                <Select
                labelId="validatorSelect_label"
                id="validatorSelect"
                value={selectedValidator}
                onChange={(e: SelectChangeEvent) => {
                    setSelectedValidator(e.target.value as TValidatorUrl);
                }}
                renderValue={(value) => <Typography sx={{ textTransform: 'capitalize' }}>{value}</Typography>}
                >
                    {
                        validators && validators.map((validator) => (
                            <MenuItem value={validator} key={validator}>
                                <ListItemText>{validator}</ListItemText>
                            </MenuItem>
                        ))
                    }
                </Select>
    )};
  