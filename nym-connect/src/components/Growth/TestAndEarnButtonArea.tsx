import React from 'react';
import { Badge, Box, Button, Tooltip } from '@mui/material';
import MonetizationOnIcon from '@mui/icons-material/MonetizationOn';
import { invoke } from '@tauri-apps/api';
import Content from './content/en.yaml';
import { useClientContext } from '../../context/main';
import { useTestAndEarnContext } from './context/TestAndEarnContext';
import { NymShipyardTheme } from '../../theme';
import { ConnectionStatusKind } from '../../types';

export const Wrapper: React.FC<{ disabled: boolean; children: React.ReactNode }> = ({ disabled, children }) => {
  if (disabled) {
    return (
      <Badge badgeContent="!" color="warning">
        <Tooltip arrow title={disabled ? Content.testAndEarn.mainWindow.button.popup.disconnected : undefined}>
          <div>{children}</div>
        </Tooltip>
      </Badge>
    );
  }
  return <>{children}</>;
};

export const TestAndEarnButtonArea: React.FC = () => {
  const clientContext = useClientContext();
  const context = useTestAndEarnContext();
  const disabled = clientContext.connectionStatus !== ConnectionStatusKind.connected;
  const pinger = React.useRef<NodeJS.Timer | null>();

  const doPing = async () => {
    if (context.clientDetails) {
      try {
        await invoke('growth_tne_ping', { clientDetails: context.clientDetails });
      } catch (_e) {
        // console.error('Failed to ping: ', e);
      }
    }
  };

  React.useEffect(() => {
    (async () => {
      if (!disabled) {
        // sleep a little until the SOCKS5 proxy connects
        setTimeout(() => {
          doPing();
        }, 1000 * 10);

        // update every 15 mins
        pinger.current = setInterval(doPing, 1000 * 60 * 15);
      } else if (pinger.current) {
        clearInterval(pinger.current);
        pinger.current = null;
      }
    })();
  }, [disabled, context.clientDetails]);

  const handleClick = async () => {
    if (!disabled) {
      await context.toggleGrowthWindow(Content.testAndEarn.popupWindow.title);
    }
  };

  return (
    <NymShipyardTheme>
      <Box justifyContent="center" display="grid">
        <Wrapper disabled={disabled}>
          <Button
            color={disabled ? 'secondary' : undefined}
            variant="contained"
            size="small"
            endIcon={<MonetizationOnIcon />}
            sx={{ width: '150px', mb: 4, opacity: disabled ? 0.4 : undefined }}
            onClick={handleClick}
          >
            {context.registration
              ? Content.testAndEarn.mainWindow.button.text.entered
              : Content.testAndEarn.mainWindow.button.text.default}
          </Button>
        </Wrapper>
      </Box>
    </NymShipyardTheme>
  );
};
