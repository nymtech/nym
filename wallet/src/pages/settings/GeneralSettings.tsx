import { Box, Divider, Stack, Typography } from '@mui/material';
import { AppVersion, ThemeSwitcher } from '../../components/Settings';

const GeneralSettings = () => (
  <Box pb={3}>
    <Stack direction="row" justifyContent="space-between" padding={3}>
      <Stack direction="column" gap={1}>
        <Typography variant="h6">Version</Typography>
        <Typography variant="caption" sx={{ color: 'nym.text.muted' }}>
          Current version of wallet and updates
        </Typography>
      </Stack>
      <Box>
        <AppVersion />
      </Box>
    </Stack>
    <Divider />
    <Stack direction="row" justifyContent="space-between" padding={3}>
      <Stack direction="column" gap={1}>
        <Typography variant="h6">Theme</Typography>
        <Typography variant="caption" sx={{ color: 'nym.text.muted' }}>
          Select the theme
        </Typography>
      </Stack>
      <Box>
        <ThemeSwitcher />
      </Box>
    </Stack>
  </Box>
);

export default GeneralSettings;
