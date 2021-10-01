import React from 'react';
import { Typography } from '@mui/material';
import { MixnodesDataGrid } from 'src/components/Mixnodes-DataGrid';

export const PageGateways: React.FC = () => {
    return (
        <>
            <Typography sx={{ marginBottom: 1 }} variant="h5">
                Gateways
            </Typography>
            <MixnodesDataGrid />
        </>
    );
};
