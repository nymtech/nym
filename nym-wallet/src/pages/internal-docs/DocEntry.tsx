import React from 'react';
import { Button, Card, CardContent, TextField } from '@mui/material';
import { invoke } from '@tauri-apps/api';

interface DocEntryProps {
  func: FunctionDef;
}

interface FunctionDef {
  name: string;
  args: ArgDef[];
}

interface ArgDef {
  name: string;
  type: string;
}

const argKey = (functionName: string, arg: string) => `${functionName}_${arg}`;

function collectArgs(functionName: string, args: ArgDef[]) {
  const invokeArgs: { [key: string]: string } = {};

  args.forEach((arg) => {
    const elem: HTMLElement | null = document.getElementById(argKey(functionName, arg.name));

    if (arg.type === 'object') {
      console.log(arg);
      invokeArgs[arg.name] = JSON.parse((elem as HTMLInputElement).value);
    } else {
      invokeArgs[arg.name] = (elem as HTMLInputElement).value || '';
    }
  });
  console.log(invokeArgs);
  return invokeArgs;
}

export const DocEntry = ({ func }: DocEntryProps) => {
  const [card, setCard] = React.useState(<Card />);

  const onClick = () => {
    invoke(func.name, collectArgs(func.name, func.args))
      .then((result) => {
        setCard(
          <Card>
            <CardContent>{JSON.stringify(result, null, 4)}</CardContent>
          </Card>,
        );
      })
      .catch((e) =>
        setCard(
          <Card>
            <CardContent>{e}</CardContent>
          </Card>,
        ),
      );
  };

  return (
    <div>
      <Button variant="contained" color="primary" size="small" disableElevation onClick={onClick}>
        {func.name}
      </Button>
      <Button variant="contained" size="small" disableElevation onClick={() => setCard(<Card />)}>
        X
      </Button>
      <div>
        {func.args.map((arg) => (
          <TextField label={arg.name} id={argKey(func.name, arg.name)} key={argKey(func.name, arg.name)} />
        ))}
      </div>
      <br />
      {card}
    </div>
  );
};
