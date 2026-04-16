import React from 'react';
import { Box, Card, CardHeader, Typography, Stack } from '@mui/material';
import { THiddenMnemonicWord, THiddenMnemonicWords, TMnemonicWords } from '../types';

export const WordTile = ({
  mnemonicWord,
  index,
  disabled,
  onClick,
  button,
}: {
  mnemonicWord: string;
  index?: number;
  disabled?: boolean;
  onClick?: boolean;
  button?: boolean;
}) => (
  <Card
    variant="outlined"
    sx={(theme) => ({
      background: button ? '#151A2C' : 'transparent',
      border: button ? `1px solid ${theme.palette.divider}` : 'none',
      cursor: onClick ? 'pointer' : 'default',
      opacity: disabled ? 0.35 : 1,
      minWidth: button ? 88 : undefined,
      width: '100%',
      transition: 'border-color 0.15s ease, opacity 0.15s ease',
      ...(button &&
        onClick && {
          '&:hover': {
            transform: 'none',
            borderColor: theme.palette.primary.main,
          },
        }),
      ...(!button && {
        boxShadow: 'none',
        '&:hover': { transform: 'none', boxShadow: 'none' },
      }),
    })}
  >
    <CardHeader
      title={mnemonicWord}
      titleTypographyProps={{
        sx: {
          fontWeight: 700,
          whiteSpace: 'normal',
          overflow: 'visible',
          textOverflow: 'clip',
          wordBreak: 'break-word',
          lineHeight: 1.25,
        },
        variant: 'body2',
        textAlign: index ? 'left' : 'center',
      }}
      sx={{
        py: 1.25,
        px: 1.5,
        '& .MuiCardHeader-content': { overflow: 'visible' },
      }}
      avatar={
        index ? (
          <Typography variant="caption" color="text.secondary" sx={{ fontWeight: 600 }}>
            {index}
          </Typography>
        ) : undefined
      }
    />
  </Card>
);

export const WordTiles = ({
  mnemonicWords,
  showIndex,
  onClick,
  buttons,
}: {
  mnemonicWords?: TMnemonicWords;
  showIndex?: boolean;
  onClick?: ({ name, index }: { name: string; index: number }) => void;
  buttons?: boolean;
}) => {
  if (mnemonicWords) {
    return (
      <Stack
        direction="row"
        flexWrap="wrap"
        useFlexGap
        spacing={2}
        justifyContent="center"
        sx={{ width: '100%', rowGap: 2, columnGap: 2 }}
      >
        {mnemonicWords.map(({ name, index, disabled }) => (
          <Box
            key={index}
            onClick={() => onClick?.({ name, index })}
            sx={{
              flex: '1 1 auto',
              minWidth: { xs: 'calc(50% - 8px)', sm: 120 },
              maxWidth: { xs: '100%', sm: 200 },
            }}
          >
            <WordTile
              mnemonicWord={name}
              index={showIndex ? index : undefined}
              onClick={!!onClick}
              disabled={disabled}
              button={buttons}
            />
          </Box>
        ))}
      </Stack>
    );
  }

  return null;
};

const HiddenWord = ({ mnemonicWord }: { mnemonicWord: THiddenMnemonicWord }) => (
  <Stack spacing={1} alignItems="center" sx={{ flex: '1 1 auto', minWidth: { xs: 100, sm: 112 }, maxWidth: 180 }}>
    <Typography variant="body2" color="text.secondary" fontWeight={600}>
      {mnemonicWord.index}.
    </Typography>
    <Box
      sx={(theme) => ({
        width: '100%',
        minHeight: 52,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        borderBottom: `2px solid ${theme.palette.divider}`,
        px: 0.5,
        pb: 0.75,
      })}
    >
      <Box sx={{ width: '100%', textAlign: 'center' }}>
        {mnemonicWord.hidden ? (
          <Typography variant="caption" color="text.disabled" sx={{ userSelect: 'none', letterSpacing: 2 }}>
            ---
          </Typography>
        ) : (
          <Typography fontWeight={700} variant="body2" sx={{ wordBreak: 'break-word', lineHeight: 1.3 }}>
            {mnemonicWord.name}
          </Typography>
        )}
      </Box>
    </Box>
  </Stack>
);

export const HiddenWords = ({ mnemonicWords }: { mnemonicWords?: THiddenMnemonicWords }) => {
  if (mnemonicWords) {
    return (
      <Stack
        direction="row"
        flexWrap="wrap"
        useFlexGap
        spacing={2}
        justifyContent="center"
        sx={{ width: '100%', mb: 1, rowGap: 2, columnGap: 2 }}
      >
        {mnemonicWords.map((mnemonicWord) => (
          <HiddenWord key={mnemonicWord.index} mnemonicWord={mnemonicWord} />
        ))}
      </Stack>
    );
  }
  return null;
};
