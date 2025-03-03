import React, {useMemo} from "react";
import {MaterialReactTable, MRT_ColumnDef, useMaterialReactTable} from "material-react-table";
import StyledLink from "../../../components/StyledLink";
import {EXPLORER_FOR_ACCOUNTS} from "@/app/api/constants";
import {splice} from "@/app/utils";
import {humanReadableCurrencyToString} from "@/app/utils/currency";
import {Typography} from "@mui/material";
import {useTheme} from "@mui/material/styles";
import WarningIcon from '@mui/icons-material/Warning';
import { Tooltip } from '@/app/components/Tooltip'

export const NodeDelegationsTable = ({ node }: { node: any}) => {
  const columns = useMemo<MRT_ColumnDef<any>[]>(() => {
    return [
      {
        id: 'nym-node-delegation-data',
        header: 'Nym Node Delegations',
        columns: [
          {
            id: 'owner',
            header: 'Delegator',
            accessorKey: 'owner',
            size: 150,
            Cell: ({ row }) => {
              return (
                <StyledLink
                  to={`${EXPLORER_FOR_ACCOUNTS}/account/${row.original.owner || "-"}`}
                  target="_blank"
                  data-testid="bond_information.node.owner"
                  color="text.primary"
                >
                  {splice(7, 29, row.original.owner)}
                </StyledLink>
              )
            },
          },
          {
            id: 'amount',
            header: 'Amount',
            accessorKey: 'amount',
            size: 150,
            Cell: ({ row }) => (
                <>{humanReadableCurrencyToString(row.original.amount)}</>
              )
          },
          {
            id: 'height',
            header: 'Delegated at height',
            accessorKey: 'height',
            size: 150,
          },
          {
            id: 'proxy',
            header: 'From vesting account?',
            accessorKey: 'proxy',
            size: 250,
            Cell: ({ row }) => {
              if(row.original.proxy?.length) {
                return (
                  <VestingDelegationWarning>Please re-delegate from your main account</VestingDelegationWarning>
                )
              }
            }
          },
        ]
      }
    ];
  }, []);

  const table = useMaterialReactTable({
    columns,
    data: node ? node.delegations : [],
  });

  return (
    <MaterialReactTable table={table} />
  );
}

export const VestingDelegationWarning = ({children, plural}: { plural?: boolean, children: React.ReactNode}) => {
  const theme = useTheme();
  return (
    <Tooltip
      text={`${plural ? 'These delegations have' : 'This delegation has'} been made with a vesting account. All tokens are liquid, if you are the delegator, please move the tokens into your main account and make the delegation from there.`}
      id="delegations"
    >
      <Typography fontSize="inherit" color={theme.palette.warning.main} display="flex" alignItems="center">
        <WarningIcon sx={{ mr: 0.5 }}/>
        {children}
      </Typography>
    </Tooltip>
  );
}