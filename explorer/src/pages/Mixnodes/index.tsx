import React from 'react';
import { GridRenderCellParams } from '@mui/x-data-grid';
import { Box } from '@mui/system';
import { Link } from 'react-router-dom';
import { Typography, TextField, MenuItem } from '@mui/material';
import Select, { SelectChangeEvent } from '@mui/material/Select';
import { useContext } from 'react';
import { UniversalDataGrid } from 'src/components/Universal-DataGrid';
import { MainContext } from 'src/context/main';
import { mixnodeToGridRow } from 'src/utils';

const columns = [
  { field: 'owner', headerName: 'Owner', width: 380, },
  {
    field: 'identity_key',
    headerName: 'Identity Key',
    width: 420,
    renderCell: (params: GridRenderCellParams) => {
      return (
        <Link to={`/network-components/mixnodes/${params.value}`} style={{ textDecoration: 'none', color: 'white', marginLeft: 16 }}>
          {params.value}
        </Link>
      )
    }
  },
  {
    field: 'bond',
    headerName: 'Bond',
    width: 130,
  },
  {
    field: 'host',
    headerName: 'IP:Port',
    width: 130,
  },
  {
    field: 'location',
    headerName: 'Location',
    width: 120,
  },
  {
    field: 'layer',
    headerName: 'Layer',
    width: 100,
    type: 'number',
  },
];

export const PageMixnodes: React.FC = () => {
  const { mixnodes, filterMixnodes, fetchMixnodes } = useContext(MainContext);
  const [rows, setRows] = React.useState();
  const [searchTerm, setSearchTerm] = React.useState('');
  const [pageSize, setPageSize] = React.useState(50);

  const resetGrid = async () => {
    // @ts-ignore
    if (mixnodes?.data) {
      const formattedRows = mixnodeToGridRow(mixnodes?.data);
      setRows(formattedRows);
    }
  }
  const filterBySearch = () => {
    if (mixnodes?.data && searchTerm !== '') {
      let results: any = [];
      mixnodes?.data.forEach((mn, i) => {
        if (mn.location && mn.location.country_name) {
          const cn = mn.location.country_name.toLowerCase();
          const idKey = mn.mix_node.identity_key.toLowerCase();
          const ownr = mn.owner.toLowerCase();
          if (
            idKey.includes(searchTerm) ||
            cn.includes(searchTerm) ||
            ownr.includes(searchTerm)
          ) {

            results.push(mn);
          }
        }
      });
      // @ts-ignore
      filterMixnodes(results)
    }
  }

  const onChange = (event: any) => {
    const st = event.target.value;
    if (mixnodes?.data && st === '') {
      resetGrid();
      // @ts-ignore
      fetchMixnodes()
    } else if (mixnodes?.data && st !== '') {
      setSearchTerm(st.toLowerCase());
    }
  }

  const handlePageSize = (event: SelectChangeEvent) => {
    const num = Number(event.target.value);
    setPageSize(num);
  };

  React.useEffect(() => {
    resetGrid();
  }, [mixnodes]);

  React.useEffect(() => {
    filterBySearch();
  }, [searchTerm]);

  return (
    <>
      <Typography sx={{ marginBottom: 1 }} variant="h5">
        Mixnodes
      </Typography>
      <Box
        sx={{
          width: '100%',
          marginBottom: 2,
          display: 'flex',
          justifyContent: 'space-between'
        }}>
        <Select
          labelId="demo-simple-select-label"
          id="demo-simple-select"
          value={JSON.stringify(pageSize)}
          onChange={handlePageSize}
          sx={{ width: 200 }}
        >
          <MenuItem value={10}>10</MenuItem>
          <MenuItem value={30}>30</MenuItem>
          <MenuItem value={50}>50</MenuItem>
          <MenuItem value={100}>100</MenuItem>
        </Select>
        <TextField
          sx={{ width: 350 }}
          placeholder="search"
          onChange={onChange}
        />

      </Box>
      <UniversalDataGrid
        loading={mixnodes?.isLoading}
        columnsData={columns}
        rows={rows}
        height={1080}
        pageSize={pageSize}
      />
    </>
  );
};
