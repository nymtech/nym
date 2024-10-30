import React, { useEffect, useState } from 'react';
import * as yup from 'yup';
import { Stack, TextField, Typography } from '@mui/material';
import { useForm } from 'react-hook-form';
import { CopyToClipboard } from 'src/components/CopyToClipboard';
import { ErrorModal } from 'src/components/Modals/ErrorModal';
import { SimpleModal } from 'src/components/Modals/SimpleModal';
import { useBondingContext } from 'src/context';
import { TBondNymNodeArgs } from 'src/types';
import { Signature } from 'src/pages/bonding/types';
import { yupResolver } from '@hookform/resolvers/yup';
import { useFormContext } from './FormContext';

const NymNodeSignature = ({
  nymnode,
  pledge,
  costParams,
  step,
  onNext,
  onClose,
  onBack,
}: {
  nymnode: TBondNymNodeArgs['nymnode'];
  pledge: TBondNymNodeArgs['pledge'];
  costParams: TBondNymNodeArgs['costParams'];
  step: number;
  onNext: () => void;
  onClose: () => void;
  onBack: () => void;
}) => {
  const [message, setMessage] = useState<string>('');
  const [error, setError] = useState<string>();
  const { generateNymNodeMsgPayload } = useBondingContext();
  const { signature, setSignature } = useFormContext();

  const yupValidationSchema = yup.object().shape({
    signature: yup.string().required('Signature is required'),
  });

  const {
    register,
    formState: { errors },
    handleSubmit,
  } = useForm<Signature>({
    defaultValues: {
      signature,
    },
    resolver: yupResolver(yupValidationSchema),
  });

  const generateMessage = async () => {
    try {
      const msg = await generateNymNodeMsgPayload({
        nymnode,
        pledge,
        costParams,
      });

      if (msg) {
        setMessage(msg);
      }
    } catch (e) {
      console.error(e);
      setError('Something went wrong while generating the payload signature');
    }
  };

  useEffect(() => {
    generateMessage();
  }, []);

  const handleNext = async () => {
    handleSubmit(onNext)();
  };

  if (error) {
    return <ErrorModal open message={error} onClose={() => {}} />;
  }

  return (
    <SimpleModal
      open
      onOk={handleNext}
      onClose={onClose}
      header="Bond Nym Node"
      subHeader={`Step ${step}/3`}
      okLabel="Next"
      onBack={onBack}
      okDisabled={Object.keys(errors).length > 0}
    >
      <Stack gap={3} mb={3}>
        <Typography variant="body1">
          Copy the message below and sign it:
          <br />
          If you are using a nym-node:
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
          onChange={(e) => setSignature(e.target.value)}
          id="outlined-multiline-static"
          name="signature"
          rows={3}
          placeholder="Paste Signature"
          helperText={errors.signature?.message}
          error={Boolean(errors.signature)}
          multiline
          fullWidth
          required
        />
      </Stack>
    </SimpleModal>
  );
};

export default NymNodeSignature;
