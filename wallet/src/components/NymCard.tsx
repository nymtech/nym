import { Card, CardContent, CardHeader, SxProps } from '@mui/material';
import { styled, Theme } from '@mui/material/styles';
import { Title } from './Title';

const CardContentNoPadding = styled(CardContent)(() => ({
  padding: 0,
  '&:last-child': {
    paddingBottom: 0,
  },
}));

export const NymCard: FCWithChildren<{
  title: string | React.ReactElement;
  subheader?: string | React.ReactChild;
  Action?: React.ReactNode;
  Icon?: React.ReactNode;
  noPadding?: boolean;
  borderless?: boolean;
  dataTestid?: string;
  sx?: SxProps;
  sxTitle?: SxProps;
  children: React.ReactNode;
}> = ({ title, subheader, Action, Icon, noPadding, borderless, children, dataTestid, sx, sxTitle }) => (
  <Card variant="outlined" sx={{ overflow: 'auto', ...(borderless && { border: 'none', dropShadow: 'none' }), ...sx }}>
    <CardHeader
      sx={{
        p: 3,
        color: (theme: Theme) => theme.palette.text.primary,
        '& .MuiCardHeader-title h5': { fontSize: '1.25rem' },
      }}
      title={<Title title={title} Icon={Icon} sx={sxTitle} />}
      subheader={subheader}
      data-testid={dataTestid || title}
      subheaderTypographyProps={{ variant: 'subtitle1' }}
      action={Action}
    />
    {noPadding ? (
      <CardContentNoPadding>{children}</CardContentNoPadding>
    ) : (
      <CardContent sx={{ p: 3, paddingTop: 0 }}>{children}</CardContent>
    )}
  </Card>
);
