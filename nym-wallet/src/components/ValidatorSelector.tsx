import React, { useEffect, useState } from 'react';
import { FormControl, InputLabel, ListItemText, MenuItem, Select, SelectChangeEvent, Typography } from '@mui/material';
// import { getValidators } from '../requests';

interface ValidatorDropdownProps {
    onChangeValidatorSelection: (validator: TValidatorOption) => void;
}

type TValidatorOption = string;

export const ValidatorSelector: React.FC<ValidatorDropdownProps> = ({
    onChangeValidatorSelection,
  }) => {
    const [validators, setValidators] = useState<TValidatorOption[] | null>(null);
    const [selectedValidator, setSelectedValidator] = useState<TValidatorOption>('');
  
    useEffect(() => {
    //   (async () => {
    //     await getValidators();
    //   })();
    setValidators(['aaa', 'bbb', 'ccc']);
    }, []);
  
    useEffect(() => {
        onChangeValidatorSelection(selectedValidator);
    }, [selectedValidator]);

    return validators &&
                <Select
                labelId="validatorSelect_label"
                id="validatorSelect"
                value={selectedValidator}
                onChange={(e: SelectChangeEvent) => {
                    setSelectedValidator(e.target.value as TValidatorOption);
                }}
                renderValue={(value) => <Typography sx={{ textTransform: 'capitalize' }}>{value}</Typography>}
                >
                    {
                        validators.map((validator) => (
                            <MenuItem value={validator} key={validator}>
                                <ListItemText>{validator}</ListItemText>
                            </MenuItem>
                        ))
                    }
                </Select>
  };
  