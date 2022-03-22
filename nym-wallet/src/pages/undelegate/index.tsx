import React, { useContext, useEffect, useState } from 'react';
import { Alert, AlertTitle, Box, Button, CircularProgress } from '@mui/material';
import { EnumRequestStatus, NymCard, RequestStatus } from '../../components';
import { UndelegateForm } from './UndelegateForm';
import { getCurrentEpoch, getPendingDelegations, getReverseMixDelegations } from '../../requests';
import { PendingUndelegate, TPagedDelegations } from '../../types';
import { ClientContext } from '../../context/main';
import { PageLayout } from '../../layouts';

export const Undelegate = () => {
  const [message, setMessage] = useState<string>();
  const [status, setStatus] = useState<EnumRequestStatus>(EnumRequestStatus.initial);
  const [isLoading, setIsLoading] = useState(true);
  const [pagedDelegations, setPagesDelegations] = useState<TPagedDelegations>();
  const [pendingUndelegations, setPendingUndelegations] = useState<PendingUndelegate[]>();

  const { clientDetails } = useContext(ClientContext);

  const initialize = async () => {
    setStatus(EnumRequestStatus.initial);
    setIsLoading(true);

    try {
      const mixnodeDelegations = await getReverseMixDelegations();
      const pendingEvents = await getPendingDelegations();
      await getCurrentEpoch();
      console.log({ mixnodeDelegations, pendingEvents });
      const pendingUndelegationEvents = pendingEvents
        .filter((evt): evt is { Undelegate: PendingUndelegate } => 'Undelegate' in evt)
        .map((e) => ({ ...e.Undelegate }));

      setPendingUndelegations(pendingUndelegationEvents);
      setPagesDelegations(mixnodeDelegations);
    } catch (e) {
      setStatus(EnumRequestStatus.error);
      setMessage(e as string);
    }

    setIsLoading(false);
  };

  useEffect(() => {
    initialize();
  }, [clientDetails]);

  return (
    <PageLayout>
      <NymCard title="Undelegate" subheader="Undelegate from a mixnode" noPadding>
        {isLoading && (
          <Box
            sx={{
              display: 'flex',
              justifyContent: 'center',
              p: 3,
            }}
          >
            <CircularProgress size={48} />
          </Box>
        )}
        <>
          {status === EnumRequestStatus.initial && pagedDelegations && (
            <UndelegateForm
              delegations={pagedDelegations?.delegations}
              pendingUndelegations={pendingUndelegations}
              onError={(m) => {
                setMessage(m);
                setStatus(EnumRequestStatus.error);
              }}
              onSuccess={(m) => {
                setMessage(m);
                setStatus(EnumRequestStatus.success);
              }}
            />
          )}
          {status !== EnumRequestStatus.initial && (
            <>
              <RequestStatus
                status={status}
                Error={
                  <Alert severity="error" data-testid="request-error">
                    An error occurred with the request: {message}
                  </Alert>
                }
                Success={
                  <Alert severity="success">
                    <AlertTitle data-testid="undelegate-success">Undelegation request complete</AlertTitle>
                    {message}
                  </Alert>
                }
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
                  variant="contained"
                  disableElevation
                  onClick={() => {
                    setStatus(EnumRequestStatus.initial);
                    initialize();
                  }}
                  size="large"
                >
                  Finish
                </Button>
              </Box>
            </>
          )}
        </>
      </NymCard>
    </PageLayout>
  );
};
