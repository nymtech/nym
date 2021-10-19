import React from 'react';
import { Grid, Typography } from '@mui/material';
import { GridRenderCellParams, GridColumnHeaderParams, GridColDef } from '@mui/x-data-grid';
import { SelectChangeEvent } from '@mui/material/Select';
import { cellStyles, UniversalDataGrid } from 'src/components/Universal-DataGrid';
import { MainContext } from 'src/context/main';
import { gatewayToGridRow } from 'src/utils';
import { GatewayResponse } from 'src/typeDefs/explorer-api';
import { TableToolbar } from 'src/components/TableToolbar';
import { ContentCard } from 'src/components/ContentCard';
import { CustomColumnHeading } from 'src/components/CustomColumnHeading';

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

    const columns: GridColDef[] = [
        {
            field: 'owner',
            renderHeader: () => <CustomColumnHeading headingTitle='Owner' />,
            width: 200,
            headerAlign: 'left',
            headerClassName: 'MuiDataGrid-header-override',
            renderCell: (params: GridRenderCellParams) => <Typography sx={cellStyles}>{params.value}</Typography>
        },
        {
            field: 'identity_key',
            renderHeader: () => <CustomColumnHeading headingTitle='Identity Key' />,
            width: 200,
            headerAlign: 'left',
            headerClassName: 'MuiDataGrid-header-override',
            renderCell: (params: GridRenderCellParams) => <Typography sx={cellStyles}>{params.value}</Typography>
        },
        {
            field: 'bond',
            renderHeader: () => <CustomColumnHeading headingTitle='Bond' />,
            width: 120,
            headerAlign: 'left',
            headerClassName: 'MuiDataGrid-header-override',
            renderCell: (params: GridRenderCellParams) => <Typography sx={cellStyles}>{params.value}</Typography>
        },
        {
            field: 'host',
            renderHeader: () => <CustomColumnHeading headingTitle='IP:Port' />,
            width: 130,
            headerAlign: 'left',
            headerClassName: 'MuiDataGrid-header-override',
            renderCell: (params: GridRenderCellParams) => <Typography sx={cellStyles}>{params.value}</Typography>
        },
        {
            field: 'location',
            renderHeader: () => <CustomColumnHeading headingTitle='Location' />,
            width: 120,
            headerAlign: 'left',
            headerClassName: 'MuiDataGrid-header-override',
            renderCell: (params: GridRenderCellParams) => {
                return (
                    <div onClick={() => handleSearch(params.value as string)} style={cellStyles}>
                        {params.value}
                    </div>
                )
            }
        },
    ];

    const handlePageSize = (event: SelectChangeEvent<string>) => {
        setPageSize(event.target.value);
    };

    if (gateways?.data) {
        return (
            <>
                <Typography sx={{ marginBottom: 3 }} variant="h5">
                    Gateways
                </Typography>

                <Grid container>
                    <Grid item xs={12} md={12} lg={12} xl={8}>
                        <ContentCard>
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
                                pageSize={pageSize}
                                pagination={gateways?.data?.length > 12}
                            />
                        </ContentCard>

                    </Grid>
                </Grid>
            </>
        );
    } else {
        return null
    }
};
