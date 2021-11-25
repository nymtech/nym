import React from 'react'
import { Button, Card, CardContent, TextField } from '@mui/material'
import { invoke } from '@tauri-apps/api'

interface DocEntryProps {
  function: FunctionDef
}

interface FunctionDef {
  name: string
  args: ArgDef[]
}

interface ArgDef {
  name: string
  type: string
}

const argKey = (functionName: string, arg: string) => `${functionName}_${arg}`

function collectArgs(functionName: string, args: ArgDef[]) {
  let invokeArgs: { [key: string]: string } = {}

  args.forEach((arg) => {
    let elem: HTMLElement | null = document.getElementById(
      argKey(functionName, arg.name),
    )

    if (arg.type === 'object') {
      console.log(arg)
      invokeArgs[arg.name] = JSON.parse((elem as HTMLInputElement).value)
    } else {
      invokeArgs[arg.name] = (elem as HTMLInputElement).value || ''
    }
  })
  console.log(invokeArgs)
  return invokeArgs
}

export const DocEntry = (props: DocEntryProps) => {
  const [card, setCard] = React.useState(<Card />)

  const onClick = () => {
    invoke(
      props.function.name,
      collectArgs(props.function.name, props.function.args),
    )
      .then((result) => {
        setCard(
          <Card>
            <CardContent>{JSON.stringify(result, null, 4)}</CardContent>
          </Card>,
        )
      })
      .catch((e) =>
        setCard(
          <Card>
            <CardContent>{e}</CardContent>
          </Card>,
        ),
      )
  }

  return (
    <div>
      <Button
        variant="contained"
        color="primary"
        size="small"
        disableElevation
        onClick={onClick}
      >
        {props.function.name}
      </Button>
      <Button
        variant="contained"
        size="small"
        disableElevation
        onClick={() => setCard(<Card />)}
      >
        X
      </Button>
      <div>
        {props.function.args.map((arg) => (
          <TextField
            label={arg.name}
            id={argKey(props.function.name, arg.name)}
            key={argKey(props.function.name, arg.name)}
          />
        ))}
      </div>
      <br />
      {card}
    </div>
  )
}
