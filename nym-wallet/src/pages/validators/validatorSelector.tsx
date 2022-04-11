import React, { useContext, useEffect, useState } from 'react';
import { FormControl, InputLabel, ListItemText, MenuItem, Select, SelectChangeEvent, Typography, useMediaQuery } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { ClientContext } from '../../context/main';

type TValidatorUrl = string;

export const ValidatorSelector: React.FC<{ onChangeValidatorSelection: (validator: TValidatorUrl) => void, type: string }> = ({
    onChangeValidatorSelection,
}) => {
    const [selectedValidator, setSelectedValidator] = useState<TValidatorUrl>('');

    const {
        validatorsUrl
    } = useContext(ClientContext);
    const theme = useTheme();
    const matches = useMediaQuery(theme.breakpoints.down('sm'));

    useEffect(() => {
        onChangeValidatorSelection(selectedValidator);
    }, [selectedValidator]);

    return (
        <FormControl fullWidth>
            <InputLabel id="validatorSelect_label">Validator API Url</InputLabel>
            <Select
                labelId="validatorSelect_label"
                id="validatorSelect"
                sx={{
                    width: '100%'
                }}
                value={selectedValidator || ''}
                label="Choose a Validator"
                onChange={(e: SelectChangeEvent) => {
                    setSelectedValidator(e.target.value as TValidatorUrl);
                }}
                renderValue={(value) => <Typography sx={{ textTransform: 'capitalize' }}>{value}</Typography>}
            >
                {
                    validatorsUrl && validatorsUrl.urls.map((validator) => (
                        <MenuItem value={validator} key={validator}>
                            <ListItemText>{validator}</ListItemText>
                        </MenuItem>
                    ))
                }
            </Select>
        </FormControl>
    )
};
