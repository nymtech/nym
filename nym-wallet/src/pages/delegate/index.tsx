import React, { useContext, useState } from 'react';
import { Alert, AlertTitle, Box, Button, Typography } from '@mui/material';
import { TransactionExecuteResult } from '@nymproject/types';
import { Link } from '@nymproject/react/link/Link';
import { DelegateForm } from './DelegateForm';
import { NymCard } from '../../components';
import { EnumRequestStatus, RequestStatus } from '../../components/RequestStatus';
import { SuccessView } from './SuccessView';
import { AppContext, urls } from '../../context/main';
import { PageLayout } from '../../layouts';

export const Delegate = () => {
  const [status, setStatus] = useState<EnumRequestStatus>(EnumRequestStatus.initial);
  const [error, setError] = useState<string>();
  const [successDetails, setSuccessDetails] = useState<{ amount: string; result: TransactionExecuteResult }>();

  const { network } = useContext(AppContext);

  return (
    <PageLayout>
      <NymCard title="Delegate" subheader="Delegate to mixnode" noPadding data-testid="delegateCard">
        <>
          {status === EnumRequestStatus.initial && (
            <Box sx={{ px: 3, mb: 1 }}>
              <Alert severity="warning">Always ensure you leave yourself enough funds to UNDELEGATE</Alert>
            </Box>
          )}
          {status === EnumRequestStatus.initial && (
            <DelegateForm
              onError={(message?: string) => {
                setStatus(EnumRequestStatus.error);
                setError(message);
              }}
              onSuccess={(details) => {
                setStatus(EnumRequestStatus.success);
                setSuccessDetails(details);
              }}
            />
          )}
          {status !== EnumRequestStatus.initial && (
            <>
              <RequestStatus
                status={status}
                Error={
                  <Alert severity="error" data-testid="delegate-error">
                    <AlertTitle>Delegation failed</AlertTitle>
                    An error occurred with the request:
                    <Box sx={{ wordBreak: 'break-word' }}>{error}</Box>
                  </Alert>
                }
                Success={<SuccessView details={successDetails} />}
              />
              <Box
                sx={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'flex-end',
                  p: 3,
                  pt: 0,
                }}
              >
                <Button
                  data-testid="finish-button"
                  size="large"
                  disableElevation
                  variant="contained"
                  onClick={() => {
                    setStatus(EnumRequestStatus.initial);
                  }}
                >
                  Finish
                </Button>
              </Box>
            </>
          )}
        </>
      </NymCard>
      <Typography sx={{ p: 3 }}>
        Checkout the{' '}
        <Link
          href={`${urls(network).networkExplorer}/network-components/mixnodes`}
          target="_blank"
          text="list of mixnodes"
        />{' '}
        for uptime and performances to help make delegation decisions
      </Typography>
    </PageLayout>
  );
};
