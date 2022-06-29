import React, { useContext, useEffect, useState } from 'react';
import { Alert, Button } from '@mui/material';
import { FeeDetails } from '@nymproject/types';
import { useNavigate } from 'react-router-dom';
import { LoadingModal } from 'src/components/Modals/LoadingModal';
import { AppContext, urls } from 'src/context';
import { unbondGateway, unbondMixNode, vestingUnbondGateway, vestingUnbondMixnode } from 'src/requests';
import { EnumNodeType } from 'src/types';
import { ConfirmationModal } from './components/ConfirmationModal';
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
          <ConfirmationModal {...confirmationDetails} onClose={() => setConfirmationDetails(undefined)} />
        )}
      </NymCard>
    </PageLayout>
  );
};

// {fee && <ConfirmationModal fee={fee} nodeType={ownership.nodeType} onPrev={resetFeeState} onConfirm={async () => {
//   setIsLoading(true);
//   try {
//     if (ownership.vestingPledge) {
//       await vestingUnbond(ownership.nodeType!);
//     } else {
//       await unbond(ownership.nodeType!);
//     }
//   } catch (e) {
//     enqueueSnackbar(`Failed to unbond ${ownership.nodeType}}`, { variant: 'error' });
//   } finally {
//     await getBondDetails();
//     await checkOwnership();
//     await userBalance.fetchBalance();
//     setIsLoading(false);
//   }
// }}/>}
// <NymCard title="Unbond" subheader="Unbond a mixnode or gateway" noPadding>
// {ownership?.hasOwnership ? (
// <>
// <Alert
// severity="info"
// data-testid="bond-noded"
// action={
// <Button
// data-testid="un-bond"
// disabled={isLoading}
// onClick={async () => {
//   setIsLoading(true)
//   if (ownership.vestingPledge) {
//     await getFee(simulateVestingUnbondMixnode);
//   } else {
//     await getFee(unbond, {type: ownership.nodeType})
//   }
// }}
// color="inherit"
// >
// Unbond
// </Button>
// }
// sx={{ m: 2 }}
// >
// {`Looks like you already have a ${ownership.nodeType} bonded.`}
// </Alert>

// <Box sx={{ p: 3 }}>
// <Fee feeType="UnbondMixnode" />
// </Box>
// </>
// ) : (
// <Alert severity="info" sx={{ m: 3 }} data-testid="no-bond">
// You do not currently have a bonded node
// </Alert>
// )}
// {isLoading && (
// <Box
// sx={{
// display: 'flex',
// justifyContent: 'center',
// p: 3,
// pt: 0,
// }}
// >
// <CircularProgress size={48} />
// </Box>
// )}
// </NymCard>
