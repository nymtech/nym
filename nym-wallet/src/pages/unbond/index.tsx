import React, { useContext, useEffect, useState } from 'react';
import { Alert, Button } from '@mui/material';
import { FeeDetails } from '@nymproject/types';
import { useNavigate } from 'react-router-dom';
import { LoadingModal } from 'src/components/Modals/LoadingModal';
import { AppContext, urls } from 'src/context';
import { unbondGateway, unbondMixNode, vestingUnbondGateway, vestingUnbondMixnode } from 'src/requests';
import { EnumNodeType } from 'src/types';
import { Confirmation } from './components/ConfirmationModal';
import { UnbondGateway } from './components/UnbondGateway';
import { UnbondMixnode } from './components/UnbondMixnode';
import { useCheckOwnership } from '../../hooks/useCheckOwnership';
import { PageLayout } from '../../layouts';
import { NymCard } from '../../components';

export const Unbond = () => {
  const { network } = useContext(AppContext);
  const { checkOwnership, ownership } = useCheckOwnership();

  const [isLoading, setIsLoading] = useState(false);
  const [confirmationDetails, setConfirmationDetails] = useState<
    { success: boolean; txUrl?: string; message?: string } | undefined
  >();
  const navigate = useNavigate();

  useEffect(() => {
    const initialiseForm = async () => {
      await checkOwnership();
    };
    initialiseForm();
  }, [checkOwnership]);

  const handleUnbondMixnode = async (isWithVestingTokens: boolean, fee: FeeDetails) => {
    let tx;
    setIsLoading(true);
    try {
      if (isWithVestingTokens) tx = await vestingUnbondMixnode(fee?.fee);
      if (!isWithVestingTokens) tx = await unbondMixNode(fee?.fee);
      setConfirmationDetails({
        success: true,
        txUrl: `${urls(network).blockExplorer}/transaction/${tx?.transaction_hash}`,
      });
    } catch (err) {
      setConfirmationDetails({
        success: false,
        message: err as string,
      });
    } finally {
      await checkOwnership();
      setIsLoading(false);
    }
  };

  const handleUnbondGateway = async (isWithVestingTokens: boolean, fee: FeeDetails) => {
    let tx;
    setIsLoading(true);
    try {
      if (isWithVestingTokens) tx = await vestingUnbondGateway(fee?.fee);
      if (!isWithVestingTokens) tx = await unbondGateway(fee?.fee);
      setConfirmationDetails({
        success: true,
        txUrl: `${urls(network).blockExplorer}/transaction/${tx?.transaction_hash}`,
      });
    } catch (err) {
      setConfirmationDetails({
        success: false,
        message: err as string,
      });
    } finally {
      await checkOwnership();
      setIsLoading(false);
    }
  };

  return (
    <PageLayout>
      <NymCard title="Unbond" subheader="Unbond a mixnode or gateway" noPadding>
        {!ownership.hasOwnership && (
          <Alert
            severity="info"
            sx={{ m: 3 }}
            data-testid="no-bond"
            action={
              <Button color="inherit" onClick={() => navigate('/bond')}>
                Bond
              </Button>
            }
          >
            You do not currently have a bonded mixnode or gateway
          </Alert>
        )}
        {ownership.hasOwnership && ownership?.nodeType === EnumNodeType.mixnode && (
          <UnbondMixnode
            isWithVestingTokens={!!ownership.vestingPledge}
            onConfirm={handleUnbondMixnode}
            onError={(err) =>
              setConfirmationDetails({
                success: false,
                message: err as string,
              })
            }
          />
        )}
        {ownership.hasOwnership && ownership?.nodeType === EnumNodeType.gateway && (
          <UnbondGateway
            isWithVestingTokens={!!ownership.vestingPledge}
            onConfirm={handleUnbondGateway}
            onError={(err) =>
              setConfirmationDetails({
                success: false,
                message: err as string,
              })
            }
          />
        )}
        {isLoading && <LoadingModal />}
        {confirmationDetails && (
          <Confirmation {...confirmationDetails} onClose={() => setConfirmationDetails(undefined)} />
        )}
      </NymCard>
    </PageLayout>
  );
};
