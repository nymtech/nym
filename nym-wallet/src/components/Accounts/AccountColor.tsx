import React from 'react';
import { CircleTwoTone } from '@mui/icons-material';
import stc from 'string-to-color';

export const AccountColor = ({ address }: { address: string }) => <CircleTwoTone sx={{ color: stc(address) }} />;
