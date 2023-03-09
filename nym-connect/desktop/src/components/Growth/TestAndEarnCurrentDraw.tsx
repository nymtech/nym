import React from 'react';
import LoadingButton from '@mui/lab/LoadingButton';
import { Alert, AlertTitle, Box, Card, CardContent, CardMedia, Link, Typography } from '@mui/material';
import { SxProps } from '@mui/system';
import { DateTime } from 'luxon';
import ReactMarkdown from 'react-markdown';
import assetAnimation from './content/assets/matrix.webp';
import { CopyToClipboard } from '../CopyToClipboard';
import { useTestAndEarnContext } from './context/TestAndEarnContext';
import { DrawEntryStatus, DrawWithWordOfTheDay } from './context/types';
import Content from './content/en.yaml';

export const TestAndEarnCurrentDrawFuture: FCWithChildren<{ draw?: DrawWithWordOfTheDay }> = ({ draw }) => {
  const startsUtc = React.useMemo(() => draw && DateTime.fromISO(draw.start_utc), [draw?.start_utc]);
  const startsIn = React.useMemo(() => {
    if (draw && startsUtc) {
      return startsUtc.toRelative();
    }
    return undefined;
  }, [draw?.start_utc]);

  if (!draw || !startsUtc) {
    return null;
  }
  return (
    <Card sx={{ mb: 2 }} elevation={10}>
      <CardContent>
        <h3>
          {Content.testAndEarn.draw.next.header} {startsIn} ⏰
        </h3>
        <p>on {startsUtc.toLocaleString(DateTime.DATETIME_FULL)}</p>
      </CardContent>
    </Card>
  );
};

export const TestAndEarnCurrentDrawEnter: FCWithChildren<{ draw?: DrawWithWordOfTheDay }> = ({ draw }) => {
  const context = useTestAndEarnContext();
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string>();
  const handleClick = async () => {
    if (!draw) {
      setError('No draw selected');
      return;
    }

    setBusy(true);
    try {
      await context.enterDraw(draw.id);
    } catch (e) {
      const message = `${e}`;
      console.error('Could not enter draw', message);
      setError(message);
    }
    setBusy(false);
  };
  return (
    <Box display="flex" flexDirection="column" alignItems="center" py={3} px={2} mx={6} my={2}>
      <Typography mb={4}>Complete today’s task for the chance to earn 1000 NYMs.</Typography>
      <LoadingButton variant="contained" size="large" loading={busy} onClick={handleClick}>
        Start task ✨
      </LoadingButton>
      {error && (
        <Box mt={2}>
          <Alert variant="filled" severity="error">
            <AlertTitle>Oh no! Something went wrong.</AlertTitle>
            {error}
          </Alert>
        </Box>
      )}
    </Box>
  );
};

export const TestAndEarnCurrentDrawEntered: FCWithChildren<{ draw?: DrawWithWordOfTheDay }> = ({ draw }) => {
  if (!draw || !draw.entry) {
    return null;
  }

  if (!draw.word_of_the_day) {
    return (
      <Alert severity="error" variant="filled">
        <AlertTitle>Oh no! Something is wrong</AlertTitle>
        Someone configured the wrong instructions for the task, you will not be able to see it until this is fixed
      </Alert>
    );
  }

  return (
    <Box
      display="flex"
      flexDirection="column"
      alignItems="center"
      sx={{ background: 'rgba(255,255,255,0.1)' }}
      py={4}
      mx={6}
      my={2}
      borderRadius={2}
    >
      <Box py={2} px={4} color="warning.light">
        <ReactMarkdown>{draw.word_of_the_day}</ReactMarkdown>
      </Box>

      <Typography>{Content.testAndEarn.task.afterText}</Typography>
      <Typography mt={2} fontFamily="monospace" fontWeight="bold" color="warning.main">
        {draw.entry.id} <CopyToClipboard iconButton light text={draw.entry.id} />
      </Typography>

      <Typography mt={2}>{Content.testAndEarn.task.beforeSocials}</Typography>
      <Typography mt={2} mx={1} textAlign="center">
        <Typography component="span" color="info.light" fontWeight="bold">
          Twitter
        </Typography>{' '}
        - remember to
        <Typography component="span" color="info.light">
          @nymproject
        </Typography>{' '}
        and use the hashtag{' '}
        <Typography component="span" color="info.light">
          #PrivacyLovesCompany
        </Typography>
      </Typography>
      <Typography mt={2}>or</Typography>
      <Typography textAlign="center" fontWeight="bold">
        Nym{' '}
        <Link target="_blank" href="https://t.me/nymchan" color="info.light">
          Telegram channel
        </Link>
      </Typography>
    </Box>
  );
};

export const TestAndEarnCurrentDraw: FCWithChildren<{
  draw?: DrawWithWordOfTheDay;
  sx?: SxProps;
}> = ({ draw, sx }) => {
  const [trigger, setTrigger] = React.useState(DateTime.now().toISO());
  const endsUtc = React.useMemo(() => draw && DateTime.fromISO(draw.end_utc), [draw?.end_utc]);
  const closesIn = React.useMemo(() => {
    if (draw && endsUtc) {
      return endsUtc.toRelative();
    }
    return undefined;
  }, [trigger, endsUtc]);

  React.useEffect(() => {
    const timer = setInterval(() => setTrigger(DateTime.now().toISO()), 1000 * 3600 * 15);
    return () => clearInterval(timer);
  }, []);

  if (draw && closesIn && endsUtc) {
    return (
      <Card sx={{ mb: 2, ...(Array.isArray(sx) ? sx : [sx]) }} elevation={10}>
        <CardContent>
          <h3>
            {"Today's task ends "}
            {closesIn}
            <Typography sx={{ opacity: 0.5 }}>
              {endsUtc.weekdayLong} {endsUtc.toLocaleString(DateTime.DATETIME_FULL)}
            </Typography>
          </h3>
          {!draw.entry && <TestAndEarnCurrentDrawEnter draw={draw} />}
          {draw.entry && <TestAndEarnCurrentDrawEntered draw={draw} />}
        </CardContent>
        <CardMedia component="img" height="150" image={assetAnimation} alt="lottery" />
      </Card>
    );
  }

  return null;
};

export const TestAndEarnCurrentDrawWithState: FCWithChildren<{
  sx?: SxProps;
}> = ({ sx }) => {
  const context = useTestAndEarnContext();

  if (
    context.draws?.current?.entry?.status === DrawEntryStatus.winner ||
    context.draws?.current?.entry?.status === DrawEntryStatus.claimed ||
    context.draws?.current?.entry?.status === DrawEntryStatus.noWin
  ) {
    return null;
  }

  if (!context.draws?.current) {
    return <TestAndEarnCurrentDrawFuture draw={context.draws?.next} />;
  }

  return <TestAndEarnCurrentDraw sx={sx} draw={context.draws.current} />;
};
