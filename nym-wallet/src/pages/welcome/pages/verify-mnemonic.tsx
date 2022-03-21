import React, { useContext, useEffect, useState } from 'react';
import { Button, CircularProgress, Stack } from '@mui/material';
import { useSnackbar } from 'notistack';
import { HiddenWords, Subtitle, Title, WordTiles } from '../components';
import { THiddenMnemonicWord, THiddenMnemonicWords, TMnemonicWord, TMnemonicWords, TPages } from '../types';
import { randomNumberBetween } from '../../../utils';
import { SignInContext } from '../context';
import { createPassword } from '../../../requests';

const numberOfRandomWords = 6;

export const VerifyMnemonic = ({ page, onNext, onPrev }: { page: TPages; onNext: () => void; onPrev: () => void }) => {
  const [randomWords, setRandomWords] = useState<TMnemonicWords>();
  const [hiddenRandomWords, setHiddenRandomWords] = useState<THiddenMnemonicWords>();
  const [currentSelection, setCurrentSelection] = useState(0);
  const [isLoading, setIsLoading] = useState(false);

  const { mnemonicWords, mnemonic, password } = useContext(SignInContext);
  const { enqueueSnackbar } = useSnackbar();

  const storePassword = async () => {
    try {
      setIsLoading(true);
      await createPassword({ mnemonic, password });
      enqueueSnackbar('Password successfully created', { variant: 'success' });
    } catch (e) {
      enqueueSnackbar(e as string, { variant: 'error' });
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    if (mnemonicWords) {
      const newRandomWords = getRandomEntriesFromArray<TMnemonicWord>(mnemonicWords, numberOfRandomWords);
      const withHiddenProperty = newRandomWords.map((word) => ({ ...word, hidden: true }));
      const shuffled = getRandomEntriesFromArray<THiddenMnemonicWord>(withHiddenProperty, numberOfRandomWords);
      setRandomWords(newRandomWords);
      setHiddenRandomWords(shuffled);
    }
  }, [mnemonicWords]);

  const revealWord = ({ name }: { name: string }) => {
    if (name === hiddenRandomWords![currentSelection].name) {
      setHiddenRandomWords((hiddenWords) =>
        hiddenWords?.map((word) => (word.name === name ? { ...word, hidden: false } : word)),
      );
      setRandomWords((argRandomWords) =>
        argRandomWords?.map((word) => (word.name === name ? { ...word, disabled: true } : word)),
      );
      setCurrentSelection((current) => current + 1);
    }
  };

  if (randomWords && hiddenRandomWords) {
    return (
      <>
        <div id={page} />
        <Title title="Verify your mnemonic" />
        <Subtitle subtitle="Select the words from your mnmonic based on their order" />
        <HiddenWords mnemonicWords={hiddenRandomWords} />
        <WordTiles
          mnemonicWords={randomWords}
          onClick={currentSelection !== numberOfRandomWords ? revealWord : undefined}
        />
        <Stack spacing={3} sx={{ width: 300 }}>
          <Button
            variant="contained"
            fullWidth
            size="large"
            disabled={currentSelection !== numberOfRandomWords || isLoading}
            onClick={async () => {
              await storePassword();
              onNext();
            }}
            endIcon={isLoading && <CircularProgress />}
          >
            Next
          </Button>
          <Button
            size="large"
            onClick={onPrev}
            fullWidth
            sx={{
              color: 'common.white',
              border: '1px solid white',
              '&:hover': { border: '1px solid white', '&:hover': { background: 'none' } },
            }}
          >
            Back
          </Button>
        </Stack>
      </>
    );
  }
  return null;
};

function getRandomEntriesFromArray<T>(arr: T[], numberOfEntries: number) {
  const init = [...arr];
  const randomEntries: T[] = [];

  while (randomEntries.length !== numberOfEntries) {
    const rand = randomNumberBetween(0, init.length - 1);
    randomEntries.push(init[rand]);
    init.splice(rand, 1);
  }

  return randomEntries;
}
