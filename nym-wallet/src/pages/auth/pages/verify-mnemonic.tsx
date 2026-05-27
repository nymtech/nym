import React, { useContext, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Button, Stack } from '@mui/material';
import { AuthContext } from 'src/context/auth';
import { randomNumberBetween } from 'src/utils';
import { HiddenWords, Subtitle, Title, WordTiles } from '../components';
import { THiddenMnemonicWord, THiddenMnemonicWords, TMnemonicWord, TMnemonicWords } from '../types';

const numberOfRandomWords = 6;

export const VerifyMnemonic = () => {
  const [randomWords, setRandomWords] = useState<TMnemonicWords>();
  const [hiddenRandomWords, setHiddenRandomWords] = useState<THiddenMnemonicWords>();
  const [currentSelection, setCurrentSelection] = useState(0);

  const { mnemonicWords } = useContext(AuthContext);
  const navigate = useNavigate();

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
      <Stack spacing={3} sx={{ width: '100%', alignItems: 'stretch' }}>
        <Title title="Verify your mnemonic" />
        <Subtitle subtitle="Select the words from your mnemonic in the order shown above" />
        <HiddenWords mnemonicWords={hiddenRandomWords} />
        <WordTiles
          mnemonicWords={randomWords}
          onClick={currentSelection !== numberOfRandomWords ? revealWord : undefined}
          buttons
        />
        <Stack
          spacing={2}
          sx={{
            width: '100%',
            maxWidth: 400,
            alignSelf: 'center',
            pt: { xs: 3, sm: 4 },
            mt: { xs: 0.5, sm: 1 },
          }}
        >
          <Button
            variant="contained"
            fullWidth
            size="large"
            disabled={currentSelection !== numberOfRandomWords}
            onClick={() => navigate('/create-password')}
            sx={{ fontWeight: 700 }}
          >
            Next
          </Button>
          <Button color="inherit" fullWidth size="large" onClick={() => navigate(-1)}>
            Back
          </Button>
        </Stack>
      </Stack>
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
