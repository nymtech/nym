import { Box, Button, Grid, Typography, useMediaQuery } from '@mui/material';
import * as React from 'react';
import Identicon from 'react-identicons';
import { useTheme } from '@mui/material/styles';
import { MixnodeRowType } from '.';
import { getMixNodeStatusText, MixNodeStatus } from './Status';
import { MixNodeDescriptionResponse } from '../../typeDefs/explorer-api';

interface MixNodeDetailProps {
  mixNodeRow: MixnodeRowType;
  mixnodeDescription: MixNodeDescriptionResponse;
}

export const MixNodeDetailSection: React.FC<MixNodeDetailProps> = ({
  mixNodeRow,
  mixnodeDescription,
}) => {
  const theme = useTheme();
  const palette = [theme.palette.text.primary];
  const matches = useMediaQuery(theme.breakpoints.down('sm'));
  const statusText = React.useMemo(
    () => getMixNodeStatusText(mixNodeRow),
    [mixNodeRow.status],
  );
  return (
    <Grid container>
      <Grid item xs={12} sm={6}>
        <Box display="flex" flexDirection="row" width="100%">
          <Box
            width={72}
            height={72}
            sx={{
              minWidth: 72,
              minHeight: 72,
              borderWidth: 1,
              borderColor: theme.palette.text.primary,
              borderStyle: 'solid',
              borderRadius: '50%',
              display: 'grid',
              placeItems: 'center',
            }}
          >
            <Identicon
              size={43}
              string={mixNodeRow.identity_key}
              palette={palette}
            />
          </Box>
          <Box ml={2}>
            <Typography fontSize={21}>{mixnodeDescription.name}</Typography>
            <Typography>
              {(mixnodeDescription.description || '').slice(0, 1000)}
            </Typography>
            <Button
              component="a"
              variant="text"
              sx={{
                mt: 4,
                borderRadius: '30px',
                fontWeight: 'bold',
                padding: 0,
              }}
              href={mixnodeDescription.link}
              target="_blank"
            >
              <Typography
                component="span"
                textOverflow="ellipsis"
                whiteSpace="nowrap"
                overflow="hidden"
                maxWidth="250px"
              >
                {mixnodeDescription.link}
              </Typography>
            </Button>
          </Box>
        </Box>
      </Grid>
      <Grid
        item
        xs={12}
        sm={6}
        display="flex"
        justifyContent="end"
        mt={matches ? 5 : undefined}
      >
        <Box display="flex" flexDirection="column">
          <Typography fontWeight="800" alignSelf="self-end">
            Node status:
          </Typography>
          <Box mt={2} alignSelf="self-end">
            <MixNodeStatus mixNodeRow={mixNodeRow} />
          </Box>
          <Typography
            mt={1}
            alignSelf="self-end"
            color={theme.palette.text.secondary}
            fontSize="smaller"
          >
            This node is {statusText} in this epoch
          </Typography>
        </Box>
      </Grid>
    </Grid>
  );
};
