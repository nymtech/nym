import React from 'react';
import { Alert, AlertTitle, Box, Checkbox, Link, Stack } from '@mui/material';
import LoadingButton from '@mui/lab/LoadingButton';
import { SxProps } from '@mui/system';
import ArrowCircleRightIcon from '@mui/icons-material/ArrowCircleRight';
import { invoke } from '@tauri-apps/api';
import { useTestAndEarnContext } from './context/TestAndEarnContext';
import { ClientId, Registration } from './context/types';

export const TestAndEarnTakePart: React.FC<{
  websiteLinkUrl: string;
  websiteLinkText: string;
  content: string;
  sx?: SxProps;
}> = ({ content, websiteLinkText, websiteLinkUrl, sx }) => {
  const [agree, setAgree] = React.useState(false);
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string>();
  const context = useTestAndEarnContext();
  const handleNext = async () => {
    try {
      setBusy(true);
      if (context.clientDetails) {
        const registration: Registration = await invoke('growth_tne_take_part');
        console.log('Registration: ', { registration });
        await context.setAndStoreRegistration(registration);
        if (registration) {
          console.log('Registered...');
        } else {
          setError('Failed to get registration details');
        }
      } else {
        setError('Failed to get client details');
      }
    } catch (e) {
      const message = `${e}`;
      console.error('An error occurred', message);
      setError(message);
      setBusy(false); // the busy state only resets on errors, for success stats, the context will navigate the window away
    }
  };
  return (
    <>
      <Stack direction="row" spacing={6} alignItems="center" sx={sx}>
        <Stack alignItems="center" direction="row">
          <Checkbox onChange={(_event, checked) => setAgree(checked)} />
          <Box color="primary.light" fontWeight="bold">
            {content}
          </Box>
        </Stack>
        <Box>
          <Link href={websiteLinkUrl} target="_blank" color="secondary" sx={{ opacity: 0.5 }}>
            {websiteLinkText}
          </Link>
        </Box>
        <LoadingButton
          loading={busy}
          disabled={!agree || busy}
          variant="contained"
          sx={{ justifySelf: 'end' }}
          endIcon={<ArrowCircleRightIcon />}
          onClick={handleNext}
        >
          Next
        </LoadingButton>
      </Stack>
      {error && (
        <Alert severity="error" variant="filled">
          <AlertTitle>Oh no! Something went wrong</AlertTitle>
          {error}
        </Alert>
      )}
    </>
  );
};
