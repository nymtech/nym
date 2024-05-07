import React, { useEffect, useState } from 'react';
import { Stack, TextField, Typography } from '@mui/material';
import { useForm } from 'react-hook-form';
import { costParamsToTauri, mixnodeToTauri } from '../utils';
import { CopyToClipboard } from '../../CopyToClipboard';
import { useBondingContext } from '../../../context';
import { Console } from '../../../utils/console';
import { ErrorModal } from '../../Modals/ErrorModal';
import { MixnodeAmount, MixnodeData, Signature } from '../../../pages/bonding/types';

const MixnodeSignatureForm = ({
  mixnode,
  amount,
  onNext,
}: {
  mixnode: MixnodeData;
  amount: MixnodeAmount;
  onNext: (data: Signature) => void;
}) => {
  const [message, setMessage] = useState<string>('');
  const [error, setError] = useState<string>();
  const { generateMixnodeMsgPayload } = useBondingContext();

  const { register, handleSubmit } = useForm<Signature>();

  const handleOnNext = (event: { detail: { step: number } }) => {
    if (event.detail.step === 3) {
      handleSubmit(onNext)();
    }
  };

  useEffect(() => {
    window.addEventListener('validate_bond_mixnode_step' as any, handleOnNext);
    return () => window.removeEventListener('validate_bond_mixnode_step' as any, handleOnNext);
  }, []);

  const generateMessage = async () => {
    try {
      setMessage(
        (await generateMixnodeMsgPayload({
          pledge: amount.amount,
          mixnode: mixnodeToTauri(mixnode),
          costParams: costParamsToTauri(amount),
          tokenPool: amount.tokenPool as 'balance' | 'locked',
        })) as string,
      );
    } catch (e) {
      Console.error(e);
      setError('Something went wrong while generating the payload signature');
    }
  };

  useEffect(() => {
    generateMessage();
  }, [mixnode, amount]);

  if (error) {
    return <ErrorModal open message={error} onClose={() => {}} />;
  }

  return (
    <Stack gap={3} mb={3}>
      <Typography variant="body1">
        Copy the message below and sign it: 
        <br />
        If you're using a nym-mixnode:
        <br />
        <code>nym-mixnode sign --id &lt;your-node-id&gt; --contract-msg &lt;payload-generated-by-the-wallet&gt;</code>
        <br />
        If you're using a nym-node:
        <br />
        <code>nym-node sign --id &lt;your-node-id&gt; --contract-msg &lt;payload-generated-by-the-wallet&gt;</code>
        <br />
        Then paste the signature in the next field.
      </Typography>
      <TextField id="outlined-multiline-static" multiline rows={7} value={message} fullWidth disabled />
      <Stack direction="row" alignItems="center" gap={1} justifyContent="end">
        <Typography fontWeight={600}>Copy Message</Typography>
        {message && <CopyToClipboard text={message} iconButton />}
      </Stack>
      <TextField
        {...register('signature')}
        id="outlined-multiline-static"
        name="signature"
        rows={3}
        placeholder="Paste Signature"
        multiline
        fullWidth
        required
      />
    </Stack>
  );
};

export default MixnodeSignatureForm;
