import React, { useEffect, useState } from 'react';
import { Alert, Box, Button, CircularProgress } from '@mui/material';
import { useSnackbar } from 'notistack';
import { BondForm } from './BondForm';
import { SuccessView } from './SuccessView';
import { NymCard } from '../../components';
import { EnumRequestStatus, RequestStatus } from '../../components/RequestStatus';
import { unbond, vestingUnbond } from '../../requests';
import { useCheckOwnership } from '../../hooks/useCheckOwnership';
import { PageLayout } from '../../layouts';

export const Bond = () => {
  const [status, setStatus] = useState(EnumRequestStatus.initial);
  const [error, setError] = useState<string>();
  const [successDetails, setSuccessDetails] = useState<{ amount: string; address: string }>();

  const { checkOwnership, ownership, isLoading } = useCheckOwnership();
  const { enqueueSnackbar } = useSnackbar();

  useEffect(() => {
    if (status === EnumRequestStatus.initial) {
      const initialiseForm = async () => {
        await checkOwnership();
      };
      initialiseForm();
    }
  }, [status, checkOwnership]);

  return (
    <PageLayout>
      <NymCard title="Bond" subheader="Bond a node or gateway" noPadding>
        {status === EnumRequestStatus.initial && (
          <Box sx={{ px: 3, mb: 1 }}>
            <Alert severity="warning">Always ensure you leave yourself enough funds to UNBOND</Alert>
          </Box>
        )}
        {ownership?.hasOwnership && (
          <Box sx={{ px: 3, mb: 3 }}>
            <Alert
              severity="info"
              action={
                <Button
                  disabled={status === EnumRequestStatus.loading}
                  onClick={async () => {
                    setStatus(EnumRequestStatus.loading);
                    try {
                      if (ownership.vestingPledge) {
                        await vestingUnbond(ownership.nodeType!);
                      } else {
                        await unbond(ownership.nodeType!);
                      }
                    } catch (e) {
                      enqueueSnackbar(`Failed to unbond ${ownership.nodeType}}`, { variant: 'error' });
                    } finally {
                      setStatus(EnumRequestStatus.initial);
                    }
                  }}
                  data-testid="unBond"
                  color="inherit"
                >
                  Unbond
                </Button>
              }
            >
              {`Looks like you already have a ${ownership.nodeType} bonded.`}
            </Alert>
          </Box>
        )}
        {status === EnumRequestStatus.loading && (
          <Box
            sx={{
              display: 'flex',
              justifyContent: 'center',
              padding: 3,
            }}
          >
            <CircularProgress size={48} />
          </Box>
        )}
        {status === EnumRequestStatus.initial && !ownership.hasOwnership && !isLoading && (
          <BondForm
            onError={(e?: string) => {
              setError(e);
              setStatus(EnumRequestStatus.error);
            }}
            onSuccess={(details) => {
              setSuccessDetails(details);
              setStatus(EnumRequestStatus.success);
            }}
            disabled={ownership?.hasOwnership}
          />
        )}
        {(status === EnumRequestStatus.error || status === EnumRequestStatus.success) && (
          <>
            <RequestStatus
              status={status}
              Success={<SuccessView details={successDetails} />}
              Error={
                <Alert severity="error" data-testid="bond-error">
                  An error occurred with the request: {error}
                </Alert>
              }
            />
            <Box
              sx={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'flex-end',
                padding: 3,
                pt: 0,
              }}
            >
              <Button
                onClick={() => {
                  setStatus(EnumRequestStatus.initial);
                }}
                variant="contained"
                color="primary"
                size="large"
                disableElevation
              >
                Finish
              </Button>
            </Box>
          </>
        )}
      </NymCard>
    </PageLayout>
  );
};
