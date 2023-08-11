import React, { useState } from 'react';
import { Link as RouterLink } from 'react-router-dom';
import { Alert, Box, Button, FormControl, Link, Snackbar, Stack, TextField, Typography } from '@mui/material';
import * as Sentry from '@sentry/react';
import { Controller, SubmitHandler, useForm } from 'react-hook-form';
import { object, string } from 'yup';
import { yupResolver } from '@hookform/resolvers/yup';
import { useClientContext } from '../../../context/main';

type FormValues = {
  email?: string;
  feedback: string;
};

const schema = object({
  email: string().email(),
  feedback: string().required().min(20).max(512),
}).required();

export const UserFeedback = () => {
  const [isBusy, setIsBusy] = useState(false);
  const { userData } = useClientContext();

  const {
    handleSubmit,
    control,
    formState: { errors },
    reset,
  } = useForm({
    defaultValues: {
      email: '',
      feedback: '',
    },
    // inferred type is fucked, so use `any` to make TS happy
    resolver: yupResolver(schema as any),
  });

  const onSubmit: SubmitHandler<FormValues> = (data) => {
    const eventId = Sentry.captureMessage('user feedback');
    Sentry.captureUserFeedback({
      event_id: eventId,
      name: 'nym',
      email: data.email,
      comments: data.feedback,
    });
    setIsBusy(true);
    reset();
  };

  const handleClose = () => {
    setIsBusy(false);
  };

  if (!userData?.monitoring) {
    return (
      <Stack mt={3}>
        <Typography variant="caption" color="warning.main" fontWeight="bold">
          The error reporting option must be enabled in order to report feedback. Turn it on{' '}
          <Link to="/menu/reporting/error-reporting" component={RouterLink} color="secondary" underline="hover">
            here
          </Link>
          .
        </Typography>
      </Stack>
    );
  }

  return (
    <Box height="100%">
      <Snackbar open={isBusy} autoHideDuration={6000} onClose={handleClose}>
        <Alert onClose={handleClose} severity="success" sx={{ width: '100%' }}>
          Feedback sent successfuly
        </Alert>
      </Snackbar>
      <Stack justifyContent="space-between" height="100%">
        <Box>
          <Typography fontWeight="bold" variant="body2">
            Send us your feedback about the app
          </Typography>
          <form onSubmit={handleSubmit(onSubmit)}>
            <FormControl sx={{ mt: 2 }} fullWidth>
              <Controller
                render={({ field }) => (
                  <TextField
                    size="small"
                    placeholder="E-mail address (optional)"
                    error={Boolean(errors.email)}
                    helperText={errors.email && errors.email.message}
                    {...field}
                  />
                )}
                name="email"
                control={control}
              />
            </FormControl>
            <FormControl sx={{ mt: 2 }} fullWidth>
              <Controller
                render={({ field }) => (
                  <TextField
                    size="small"
                    placeholder="Feedback text"
                    rows={8}
                    multiline
                    required
                    error={Boolean(errors.feedback)}
                    helperText={errors.feedback && errors.feedback.message}
                    {...field}
                  />
                )}
                name="feedback"
                control={control}
              />
            </FormControl>
            <Stack>
              <Button color="primary" variant="contained" size="medium" type="submit" sx={{ mt: 2 }} disabled={isBusy}>
                Send
              </Button>
            </Stack>
          </form>
        </Box>
      </Stack>
    </Box>
  );
};
