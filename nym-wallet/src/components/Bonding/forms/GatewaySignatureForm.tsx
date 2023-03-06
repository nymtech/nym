import React, { useEffect, useState } from 'react';
import { Stack, TextField, Typography } from '@mui/material';
import { gatewayToTauri } from '../utils';
import { CopyToClipboard } from '../../CopyToClipboard';
import { useBondingContext } from '../../../context';
import { Console } from '../../../utils/console';
import { ErrorModal } from '../../Modals/ErrorModal';
import { GatewayData, GatewayAmount } from '../../../pages/bonding/types';

const GatewaySignatureForm = ({
  gateway,
  amount,
  onSignatureChange,
  onNext,
}: {
  gateway: GatewayData;
  amount: GatewayAmount;
  onNext: () => void;
  onSignatureChange: (signature: string) => void;
}) => {
  const [message, setMessage] = useState<string>();
  const [signature, setSignature] = useState<string>();
  const [error, setError] = useState<string>();
  const { generateGatewayMsgPayload } = useBondingContext();

  const handleOnNext = () => {
    onNext();
  };

  useEffect(() => {
    window.addEventListener('validate_bond_gateway_step' as any, handleOnNext);
    return () => window.removeEventListener('validate_bond_gateway_step' as any, handleOnNext);
  }, []);

  useEffect(() => {
    if (signature) {
      onSignatureChange(signature);
    }
  }, [signature]);

  const generateMessage = async () => {
    try {
      setMessage(
        await generateGatewayMsgPayload({
          pledge: amount.amount,
          gateway: gatewayToTauri(gateway),
        }),
      );
    } catch (e) {
      Console.error(e);
      setError('Something went wrong while generating the payload signature');
    }
  };

  useEffect(() => {
    generateMessage();
  }, [gateway, amount]);

  if (error) {
    return <ErrorModal open message={error} onClose={() => {}} />;
  }

  return (
    <Stack gap={3} mb={3}>
      <Typography variant="body2">
        Copy below message and sign it with your gateway using `` command. Then paste the signature in the next field.
      </Typography>
      <TextField id="outlined-multiline-static" multiline rows={6} value={message} fullWidth disabled />
      <Stack direction="row" alignItems="center" gap={1} justifyContent="end">
        <Typography>Copy Message</Typography>
        <CopyToClipboard text={message} iconButton />
      </Stack>
      <TextField
        id="outlined-multiline-static"
        multiline
        rows={8}
        placeholder="Paste Signature"
        fullWidth
        value={signature}
        onChange={(e) => setSignature(e.target.value)}
      />
    </Stack>
  );
};

export default GatewaySignatureForm;
