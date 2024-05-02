import * as React from 'react';
import { Box, Button, Grid, Typography, useTheme } from '@mui/material';
import Identicon from 'react-identicons';
import { useIsMobile } from '@src/hooks/useIsMobile';
import { MixnodeRowType } from '.';
import { getMixNodeStatusText, MixNodeStatus } from './Status';
import { MixNodeDescriptionResponse } from '../../typeDefs/explorer-api';

interface MixNodeDetailProps {
  mixNodeRow: MixnodeRowType;
  mixnodeDescription: MixNodeDescriptionResponse;
}

export const MixNodeDetailSection: React.FC<MixNodeDetailProps> = ({ mixNodeRow, mixnodeDescription }) => {
  const theme = useTheme();
  const isMobile = useIsMobile();
  const statusText = React.useMemo(() => getMixNodeStatusText(mixNodeRow.status), [mixNodeRow.status]);

  const title = mixnodeDescription.name || mixnodeDescription.moniker || "Unknown Node";
  const description = mixnodeDescription.description || mixnodeDescription.details || "No description available.";
  const link = mixnodeDescription.link || mixnodeDescription.website || '#';

  return (
    <Grid container>
      <Grid item xs={12} md={6}>
        <Box display="flex" flexDirection={isMobile ? 'column' : 'row'} width="100%">
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
            <Identicon size={43} string={mixNodeRow.identity_key} />
          </Box>
          <Box ml={isMobile ? 0 : 2} mt={isMobile ? 2 : 0}>
            <Typography fontSize={21}>{title}</Typography>
            <Typography>{description.slice(0, 1000)}</Typography>
            <Button
              component="a"
              variant="text"
              sx={{
                mt: isMobile ? 2 : 4,
                borderRadius: '30px',
                fontWeight: 600,
                padding: 0,
              }}
              href={link}
              target="_blank"
            >
              <Typography
                component="span"
                textOverflow="ellipsis"
                whiteSpace="nowrap"
                overflow="hidden"
                maxWidth="250px"
              >
                Visit Node
              </Typography>
            </Button>
          </Box>
        </Box>
      </Grid>
      <Grid
        item
        xs={12}
        md={6}
        display="flex"
        justifyContent={isMobile ? 'start' : 'end'}
        mt={isMobile ? 3 : undefined}
      >
        <Box display="flex" flexDirection="column">
          <Typography fontWeight="600" alignSelf={isMobile ? 'start' : 'self-end'}>
            Node status:
          </Typography>
          <Box mt={2} alignSelf={isMobile ? 'start' : 'self-end'}>
            <MixNodeStatus status={mixNodeRow.status} />
          </Box>
          <Typography
            mt={1}
            alignSelf={isMobile ? 'start' : 'self-end'}
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