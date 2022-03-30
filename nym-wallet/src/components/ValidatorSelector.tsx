import React, { useContext, useEffect, useState } from 'react';
import { ListItemText, MenuItem, Select, SelectChangeEvent, Typography, useMediaQuery } from '@mui/material';
import { useTheme } from '@mui/material/styles';
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
    const theme = useTheme();
    const matches = useMediaQuery(theme.breakpoints.down('sm'));

    useEffect(() => {
        (async () => {
            if (network) {
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
            sx={{
                width: matches ? 'auto' : 300
            }}
            value={selectedValidator || 'choose validator url'}
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
    )
};
