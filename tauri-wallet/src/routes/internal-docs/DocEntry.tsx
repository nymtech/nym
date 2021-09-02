import React, { useState } from 'react'
import {
    Box,
    Button,
    Card,
    Checkbox,
    Divider,
    FormControlLabel,
    Grid,
    InputAdornment,
    TextField,
    Theme,
} from '@material-ui/core'
import { useTheme } from '@material-ui/styles'
import { invoke } from '@tauri-apps/api'
import CardContent from '@material-ui/core/CardContent';

interface DocEntryProps {
    function: FunctionDef;
}

interface FunctionDef {
    name: string,
    args: ArgDef[]
}

interface ArgDef {
    name: string,
    type: string
}

const argKey = (functionName: string, arg: string) => `${functionName}_${arg}`

function collectArgs(functionName: string, args: ArgDef[]) {
    let invokeArgs = {}
    for (let arg of args) {
        if (arg.type === 'object') {
            invokeArgs[arg.name] = JSON.parse(document.getElementById(argKey(functionName, arg.name)).value)
        } else {
            invokeArgs[arg.name] = document.getElementById(argKey(functionName, arg.name)).value
        }
    }
    return invokeArgs
}

export const DocEntry = (props: DocEntryProps) => {
    const [card, setCard] = React.useState(<Card />)
    const theme: Theme = useTheme()

    const onClick = () => {
        invoke(props.function.name, collectArgs(props.function.name, props.function.args)).then(
            (result) => {
                setCard(<Card><CardContent>{JSON.stringify(result, null, 4)}</CardContent></Card>)
            }
        ).catch(
            (e) => setCard(<Card><CardContent>{e}</CardContent></Card>)
        )
    }

    let fields = []
    for (let arg of props.function.args) {
        fields.push(<TextField
            label={arg.name}
            id={argKey(props.function.name, arg.name)}
            key={argKey(props.function.name, arg.name)} />
        )
    }
    return (
        <div>
            <Button variant="contained"
                color="primary"
                size="small"
                disableElevation
                onClick={onClick}>
                {props.function.name}
            </Button>
            <Button
                variant="contained"
                size="small"
                disableElevation
                onClick={() => setCard(<Card />)}>X
            </Button>
            <div>{fields}</div>
            <br />
            {card}
        </div>

    )
}
