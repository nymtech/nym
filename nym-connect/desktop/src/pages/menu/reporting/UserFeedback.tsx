import React, { useState } from 'react';
import { Alert, Box, Button, FormControl, Snackbar, Stack, TextField, Typography } from '@mui/material';
import * as Sentry from '@sentry/browser';
import { Controller, SubmitHandler, useForm } from 'react-hook-form';
import { object, string } from 'yup';
import { yupResolver } from '@hookform/resolvers/yup';

type FormValues = {
  email?: string;
  feedback: string;
};

const schema = object({
  email: string().email(),
  feedback: string().required().min(20),
}).required();

export const UserFeedback = () => {
  const [isBusy, setIsBusy] = useState(false);

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
    Sentry.captureUserFeedback({
      event_id: 'user_feedback',
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

  return (
    <Box height="100%">
      <Snackbar open={isBusy} autoHideDuration={6000} onClose={handleClose}>
        <Alert onClose={handleClose} severity="success" sx={{ width: '100%' }}>
          Feedback sent successfuly
        </Alert>
      </Snackbar>
      <Stack justifyContent="space-between" height="100%">
        <Box>
          <Typography fontWeight="bold" variant="body2" mb={2}>
            Send us your feedback about the app
          </Typography>
          <form onSubmit={handleSubmit(onSubmit)}>
            <FormControl fullWidth>
              <Controller
                render={({ field }) => (
                  <TextField
                    size="small"
                    placeholder="E-mail address (optional)"
                    sx={{ mt: 1 }}
                    error={Boolean(errors.email)}
                    helperText={errors.email && errors.email.message}
                    {...field}
                  />
                )}
                name="email"
                control={control}
              />
              <Controller
                render={({ field }) => (
                  <TextField
                    size="small"
                    placeholder="Feedback text"
                    sx={{ mt: 2 }}
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
              <Button variant="contained" size="medium" type="submit" sx={{ mt: 2 }} disabled={isBusy}>
                Send
              </Button>
            </FormControl>
          </form>
        </Box>
      </Stack>
    </Box>
  );
};
