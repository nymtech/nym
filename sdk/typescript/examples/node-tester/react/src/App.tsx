import React, { useState } from 'react';
import {
  Button,
  Card,
  CardActions,
  CardContent,
  CardHeader,
  CircularProgress,
  Grid,
  List,
  ListItem,
  ListItemText,
  TextField,
  Typography,
} from '@mui/material';
import { NodeTestResultResponse } from '@nymproject/sdk';
import { ScoreIndicator } from 'src/components/ScoreIndicator';
import { useNodeTesterClient } from 'src/hooks/useNodeTesterClient';
import { BasicPageLayout } from 'src/layouts';
import { TestStatusLabel } from 'src/components/TestStatusLabel';
import Icon from '../../../../../../assets/appicon/appicon.png';

export const App = () => {
  const { testState, error, testNode, disconnectFromGateway, reconnectToGateway } = useNodeTesterClient();
  const [mixnodeIdentity, setMixnodeIdentity] = useState<string>('');
  const [results, setResults] = React.useState<NodeTestResultResponse>();

  console.log({ testState, error, testNode });

  const handleTestNode = async () => {
    setResults(undefined);
    try {
      const result = await testNode(mixnodeIdentity);
      setResults(result);
    } catch (e) {
      console.error(e);
    }
  };

  return (
    <BasicPageLayout>
      <Card variant="outlined" sx={{ mt: 15, p: 4 }}>
        <CardHeader
          title={<Typography variant="h6">Nym Mixnode Testnet Node Tester</Typography>}
          action={<TestStatusLabel state={testState} />}
          avatar={<img src={Icon} width={40} />}
        />
        <CardContent sx={{ mb: 2 }}>
          <Grid container spacing={2}>
            <Grid item xs={12} sm={6}>
              <ScoreIndicator score={results?.score || 0} />
            </Grid>
            <Grid item xs={12} sm={6}>
              <List>
                <ListItem>
                  <ListItemText primary="Packets sent" secondary={results?.sentPackets.toString() || '-'} />
                </ListItem>
                <ListItem>
                  <ListItemText primary="Packets received" secondary={results?.receivedPackets.toString() || '-'} />
                </ListItem>
                <ListItem>
                  <ListItemText
                    primary="Duplicate packets received"
                    secondary={results?.duplicatePackets.toString() || '-'}
                  />
                </ListItem>
              </List>
            </Grid>
          </Grid>
        </CardContent>
        <CardActions>
          <Grid container spacing={2}>
            <Grid item xs={12}>
              <TextField
                label="Enter a Mixnode Identity to test"
                value={mixnodeIdentity}
                onChange={(e) => {
                  setMixnodeIdentity(e.target.value);
                }}
                fullWidth
              />
            </Grid>
            <Grid item xs={12} sm={4}>
              <Button
                disabled={!disconnectFromGateway || testState === 'Disconnected' || testState === 'Testing'}
                onClick={disconnectFromGateway}
                variant="outlined"
                disableElevation
                size="large"
                fullWidth
                sx={{ mr: 2 }}
              >
                Disconnect
              </Button>
            </Grid>
            <Grid item xs={12} sm={4}>
              <Button
                disabled={!reconnectToGateway || testState === 'Ready' || testState === 'Testing'}
                onClick={reconnectToGateway}
                variant="outlined"
                disableElevation
                size="large"
                fullWidth
                sx={{ mr: 2 }}
              >
                Reconnect
              </Button>
            </Grid>
            <Grid item xs={12} sm={4}>
              <Button
                disabled={!testNode || !mixnodeIdentity || testState === 'Testing' || testState === 'Disconnected'}
                onClick={handleTestNode}
                variant="contained"
                disableElevation
                fullWidth
                size="large"
                endIcon={testState === 'Testing' && <CircularProgress size={25} />}
              >
                Start test
              </Button>
            </Grid>
          </Grid>
        </CardActions>
      </Card>
    </BasicPageLayout>
  );
};
