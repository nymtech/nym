import React, { useContext, useEffect, useState } from 'react';
import { Alert, AlertTitle, Box, Button, CircularProgress, Grid, IconButton } from '@mui/material';
import { ArrowDropDown, ArrowDropUp } from '@mui/icons-material';
import { EnumRequestStatus, NymCard, RequestStatus } from '../../components';
import { UndelegateForm } from './UndelegateForm';
import { AppContext } from '../../context/main';
import {
  getCurrentEpoch,
  getPendingDelegations,
  getPendingVestingDelegations,
  getReverseMixDelegations,
} from '../../requests';
import { DelegationResult, Epoch, PendingUndelegate, TPagedDelegations } from '../../types';
import { PageLayout } from '../../layouts';
import { removeObjectDuplicates } from '../../utils';
import { PendingEvents } from './PendingEvents';

export const Undelegate = () => {
  const [message, setMessage] = useState<string>();
  const [status, setStatus] = useState<EnumRequestStatus>(EnumRequestStatus.initial);
  const [isLoading, setIsLoading] = useState(true);
  const [pagedDelegations, setPagesDelegations] = useState<TPagedDelegations>();
  const [pendingUndelegations, setPendingUndelegations] = useState<PendingUndelegate[]>();
  const [pendingDelegations, setPendingDelegations] = useState<DelegationResult[]>();
  const [currentEndEpoch, setCurrentEndEpoch] = useState<Epoch['end']>();
  const [showPendingDelegations, setShowPendingDelegations] = useState(false);

  const { clientDetails } = useContext(AppContext);

  const refresh = async () => {
    const mixnodeDelegations = await getReverseMixDelegations();
    const pendingEvents = await getPendingDelegations();
    const pendingVestingEvents = await getPendingVestingDelegations();
    const pendingUndelegationEvents = [...pendingEvents, ...pendingVestingEvents]
      .filter((evt): evt is { Undelegate: PendingUndelegate } => 'Undelegate' in evt)
      .map((e) => ({ ...e.Undelegate }));
    const pendingDelegationEvents = [...pendingEvents, ...pendingVestingEvents]
      .filter((evt): evt is { Delegate: DelegationResult } => 'Delegate' in evt)
      .map((e) => ({ ...e.Delegate }));
    const epoch = await getCurrentEpoch();

    setCurrentEndEpoch(epoch.end);
    setPendingUndelegations(pendingUndelegationEvents);
    setPendingDelegations(pendingDelegationEvents);
    setPagesDelegations({
      ...mixnodeDelegations,
      delegations: removeObjectDuplicates(mixnodeDelegations.delegations, 'node_identity'),
    });
  };

  const initialize = async () => {
    setStatus(EnumRequestStatus.initial);
    setIsLoading(true);

    try {
      await refresh();
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
      <Grid container direction="column" spacing={2}>
        <Grid item>
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
                  currentEndEpoch={currentEndEpoch}
                  onError={(m) => {
                    setMessage(m);
                    setStatus(EnumRequestStatus.error);
                    refresh();
                  }}
                  onSuccess={(m) => {
                    setMessage(m);
                    setStatus(EnumRequestStatus.success);
                    refresh();
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
        </Grid>
        {pendingDelegations?.length && (
          <Grid item>
            <NymCard
              title="Pending events"
              subheader="Pending delegations"
              noPadding
              Action={
                <IconButton onClick={() => setShowPendingDelegations((show) => !show)}>
                  {!showPendingDelegations ? <ArrowDropDown /> : <ArrowDropUp />}
                </IconButton>
              }
            >
              {pendingDelegations ? (
                <PendingEvents pendingDelegations={pendingDelegations} show={showPendingDelegations} />
              ) : (
                <div>No pending delegations</div>
              )}
            </NymCard>
          </Grid>
        )}
      </Grid>
    </PageLayout>
  );
};
