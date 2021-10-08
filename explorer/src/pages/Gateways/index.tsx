import React from 'react';
import { Typography } from '@mui/material';
import { SelectChangeEvent } from '@mui/material/Select';
import { UniversalDataGrid } from 'src/components/Universal-DataGrid';
import { MainContext } from 'src/context/main';
import { gatewayToGridRow } from 'src/utils';
import { GatewayResponse } from 'src/typeDefs/explorer-api';
import { TableToolbar } from 'src/components/TableToolbar';

const columns = [
    { field: 'owner', headerName: 'Owner', width: 380, },
    {
        field: 'identity_key',
        headerName: 'Identity Key',
        width: 420,
    },
    {
        field: 'bond',
        headerName: 'Bond',
        width: 130,
    },
    {
        field: 'host',
        headerName: 'IP:Port',
        width: 170,
    },
    {
        field: 'location',
        headerName: 'Location',
        width: 120,
    }
];

export const PageGateways: React.FC = () => {
    const { gateways } = React.useContext(MainContext);
    const [filteredGateways, setFilteredGateways] = React.useState<GatewayResponse>([])
    const [pageSize, setPageSize] = React.useState<string>("50");

    const handleSearch = (event: React.ChangeEvent<HTMLInputElement>) => {
        const st = event.target.value.toLowerCase();
        if (st === '' && gateways?.data) {
            setFilteredGateways(gateways?.data)
        } else {
            const filtered = gateways?.data?.filter((g) => {
                if (
                    g.gateway.location.toLowerCase().includes(st) ||
                    g.gateway.identity_key.toLocaleLowerCase().includes(st) ||
                    g.owner.toLowerCase().includes(st)
                ) {
                    return g;
                }
            })
            if (filtered) {
                setFilteredGateways(filtered)
            }
        }
    }

    const handlePageSize = (event: SelectChangeEvent<string>) => {
        setPageSize(event.target.value);
    };

    React.useEffect(() => {
        if (gateways?.data) {
            setFilteredGateways(gateways?.data)
        }
    }, [gateways]);

    return (
        <>
            <Typography sx={{ marginBottom: 1 }} variant="h5">
                Gateways
            </Typography>
            <TableToolbar
                onChangeSearch={handleSearch}
                onChangePageSize={handlePageSize}
                pageSize={pageSize}
            />
            <UniversalDataGrid
                loading={gateways?.isLoading}
                columnsData={columns}
                rows={gatewayToGridRow(filteredGateways)}
                height={600}
                pageSize={pageSize}
            />
        </>
    );
};
