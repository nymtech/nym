import { Alert } from '@mui/material';

export const Error = ({ message }: { message: string }) => (
  <Alert
    severity="error"
    variant="outlined"
    data-testid="error"
    sx={{ color: 'error.light', width: '100%', fontWeight: 600 }}
  >
    {message}
  </Alert>
);
