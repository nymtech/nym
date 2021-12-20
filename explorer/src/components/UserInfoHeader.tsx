import * as React from 'react';
import { Box, Button, Grid, Typography, useMediaQuery } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import {
  AccountCircleOutlined as AccountCircleOutlinedIcon,
  PauseCircleOutline as PauseCircleOutlineIcon,
  CircleOutlined as CircleOutlinedIcon,
  CheckCircleOutlined as CheckCircleOutlinedIcon,
} from '@mui/icons-material';

interface UserInfoHeaderProps {
  status: 'active' | 'inactive' | 'stand-by';
  name?: string;
  description?: string;
  profilePic?: string;
}
export const UserInfoHeader: React.FC<UserInfoHeaderProps> = ({
  status,
  name,
  description,
  profilePic,
}) => {
  const theme = useTheme();
  const isMobile = useMediaQuery(theme.breakpoints.down('md'));

  const dynamicColor = () => {
    let c = theme.palette.nym.success;
    if (status === 'inactive') {
      c = 'inherit';
    }
    if (status === 'stand-by') {
      c = theme.palette.nym.info;
    }
    return c;
  };

  return (
    <>
      <Box sx={{ mt: 2, mb: 2 }}>
        <Grid container>
          <Grid item xs={12} md={6}>
            <Box
              sx={{
                display: 'flex',
                flexDirection: isMobile ? 'column' : 'row',
                alignItems: 'center',
              }}
            >
              {profilePic && (
                <img src={profilePic} alt="my pic" height={66} width={66} />
              )}
              <AccountCircleOutlinedIcon
                style={{ height: '66px', width: '66px' }}
              />
              <Typography
                variant="h6"
                sx={{ fontWeight: 600, ml: isMobile ? 0 : 2 }}
              >
                Mixnode
              </Typography>
            </Box>
          </Grid>
          <Grid item xs={12} md={6}>
            <Box
              sx={{
                display: 'flex',
                flexDirection: 'column',
                justifyContent: 'flex-end',
              }}
            >
              {!isMobile && (
                <Typography
                  variant="body1"
                  sx={{ fontWeight: 600, textAlign: 'end' }}
                >
                  Node Status:
                </Typography>
              )}
              <Typography
                variant="body1"
                sx={{
                  textAlign: 'end',
                  color: dynamicColor(),
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: isMobile ? 'center' : 'flex-end',
                  mt: isMobile ? 2 : 0,
                  mb: isMobile ? 2 : 0,
                }}
              >
                {status === 'active' && <CheckCircleOutlinedIcon />}
                {status === 'inactive' && <CircleOutlinedIcon />}
                {status === 'stand-by' && <PauseCircleOutlineIcon />}
                &nbsp;{status}
              </Typography>
              {status === 'stand-by' && (
                <Typography
                  variant="body1"
                  sx={{
                    color: 'darkgrey',
                    textAlign: isMobile ? 'center' : 'end',
                  }}
                >
                  This node is on standy by in this epoch
                </Typography>
              )}
            </Box>
          </Grid>
          <Grid item xs={12} sx={{ mt: isMobile ? 4 : 2 }}>
            <Typography sx={{ fontSize: 21 }}>
              {name || 'This node has not yet set a name'}
            </Typography>
          </Grid>
          <Grid item xs={12} sx={{ mb: isMobile ? 2 : 2 }}>
            <Typography sx={{ fontSize: 16 }}>
              {description || 'This node has not yet set a description'}
            </Typography>
          </Grid>
          <Grid item xs={12}>
            <Box
              sx={{
                display: 'flex',
                flexDirection: isMobile ? 'column' : 'row',
                alignItems: 'center',
              }}
            >
              <Button
                variant="contained"
                onClick={() => null}
                sx={{
                  background:
                    'linear-gradient(90deg, #F4731B 1.05%, #F12D50 100%)',
                  borderRadius: 30,
                  fontWeight: 800,
                }}
              >
                THIS IS A LINK
              </Button>
            </Box>
          </Grid>
        </Grid>
      </Box>
    </>
  );
};

UserInfoHeader.defaultProps = {
  name: undefined,
  description: undefined,
  profilePic: undefined,
};
