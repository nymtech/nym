import React from 'react';
import { printableCoin } from '@nymproject/nym-validator-client';
import { Typography } from '@mui/material';
import { GridRenderCellParams } from '@mui/x-data-grid';
import { Link as RRDLink } from 'react-router-dom';
import { Link as MuiLink } from '@mui/material';
import { SelectChangeEvent } from '@mui/material/Select';
import { UniversalDataGrid } from 'src/components/Universal-DataGrid';
import { MainContext } from 'src/context/main';
import { gatewayToGridRow } from 'src/utils';
import { GatewayResponse } from 'src/typeDefs/explorer-api';
import { TableToolbar } from 'src/components/TableToolbar';

export const PageGateways: React.FC = () => {
    const { gateways } = React.useContext(MainContext);
    const [filteredGateways, setFilteredGateways] = React.useState<GatewayResponse>([])
    const [pageSize, setPageSize] = React.useState<string>("50");
    const [searchTerm, setSearchTerm] = React.useState<string>('');

    const handleSearch = (str: string) => {
        setSearchTerm(str.toLowerCase())
    }

    React.useEffect(() => {
        if (searchTerm === '' && gateways?.data) {
            setFilteredGateways(gateways?.data)
        } else {
            const filtered = gateways?.data?.filter((g) => {
                if (
                    g.gateway.location.toLowerCase().includes(searchTerm) ||
                    g.gateway.identity_key.toLocaleLowerCase().includes(searchTerm) ||
                    g.owner.toLowerCase().includes(searchTerm)
                ) {
                    return g;
                }
            })
            if (filtered) {
                setFilteredGateways(filtered)
            }
        }
    }, [searchTerm, gateways?.data])

    const linkStyles = {
        color: 'inherit',
        textDecoration: 'none',
        marginLeft: 2,
    }

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
            width: 180,
            renderCell: (params: GridRenderCellParams) => {
                const bondAsPunk = printableCoin({ amount: params.value as string, denom: 'upunk' })
                return (
                    <Typography sx={linkStyles}>
                        {bondAsPunk}
                    </Typography>
                )
            }
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
            renderCell: (params: GridRenderCellParams) => {
                return (
                    <div onClick={() => handleSearch(params.value as string)} style={{ textDecoration: 'none', color: 'inherit', marginLeft: 16 }}>
                        {params.value}
                    </div>
                )
            }
        },
    ];

    const handlePageSize = (event: SelectChangeEvent<string>) => {
        setPageSize(event.target.value);
    };

    return (
        <>
            <Typography sx={{ marginBottom: 1 }} variant="h5">
                Gateways
            </Typography>
            <TableToolbar
                onChangeSearch={handleSearch}
                onChangePageSize={handlePageSize}
                pageSize={pageSize}
                searchTerm={searchTerm}
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
