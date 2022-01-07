import { Box, Button, Grid, Typography } from '@mui/material';
import * as React from 'react';
import Identicon from 'react-identicons';
import { useTheme } from '@mui/material/styles';
import { MixnodeRowType } from '../MixNodes';
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
  const statusText = React.useMemo(
    () => getMixNodeStatusText(mixNodeRow),
    [mixNodeRow.status],
  );
  return (
    <>
      <Grid container>
        <Grid
          item
          xs={6}
          display="flex"
          flexDirection="row"
          alignItems="center"
        >
          <Box
            width={72}
            height={72}
            sx={{
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
          <Typography ml={2} fontSize={21}>
            Mixnode
          </Typography>
        </Grid>
        <Grid item xs={6} display="flex" justifyContent="end">
          <Box display="flex" flexDirection="column">
            <Typography fontWeight="800" alignSelf="self-end">
              Node status:
            </Typography>
            <Typography mt={2} alignSelf="self-end">
              <MixNodeStatus mixNodeRow={mixNodeRow} />
            </Typography>
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
      <Grid container>
        <Grid item xs={12}>
          <Typography fontWeight="bold">{mixnodeDescription.name}</Typography>
          <Typography fontSize="smaller">
            {mixnodeDescription.description}
          </Typography>
          <Button
            component="a"
            variant="contained"
            sx={{ mt: 4, borderRadius: '30px', fontWeight: 'bold' }}
            href={mixnodeDescription.link}
          >
            <Typography component="span" paddingX={2}>
              {mixnodeDescription.link}
            </Typography>
          </Button>
        </Grid>
      </Grid>
    </>
  );
};
