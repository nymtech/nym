import React, { useContext, useEffect, useState } from 'react';
import { useHistory } from 'react-router-dom';
import { Button, Stack } from '@mui/material';
import { HiddenWords, Subtitle, Title, WordTiles } from '../components';
import { THiddenMnemonicWord, THiddenMnemonicWords, TMnemonicWord, TMnemonicWords } from '../types';
import { randomNumberBetween } from '../../../utils';
import { SignInContext } from '../context';

const numberOfRandomWords = 6;

export const VerifyMnemonic = () => {
  const [randomWords, setRandomWords] = useState<TMnemonicWords>();
  const [hiddenRandomWords, setHiddenRandomWords] = useState<THiddenMnemonicWords>();
  const [currentSelection, setCurrentSelection] = useState(0);

  const { mnemonicWords } = useContext(SignInContext);
  const history = useHistory();

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
        <Title title="Verify your mnemonic" />
        <Subtitle subtitle="Select the words from your mnmonic based on their order" />
        <HiddenWords mnemonicWords={hiddenRandomWords} />
        <WordTiles
          mnemonicWords={randomWords}
          onClick={currentSelection !== numberOfRandomWords ? revealWord : undefined}
          buttons
        />
        <Stack spacing={3} sx={{ width: 300 }}>
          <Button
            variant="contained"
            fullWidth
            size="large"
            disabled={currentSelection !== numberOfRandomWords}
            onClick={() => history.push('/create-password')}
          >
            Next
          </Button>
          <Button color="inherit" fullWidth size="large" onClick={() => history.goBack()}>
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
