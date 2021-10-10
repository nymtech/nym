import * as React from 'react';
import { useTheme } from '@mui/material/styles';

type SVGWrapperProps = {
    SVGElement?: React.JSXElementConstructor<any>
    children: any
    height: number
    width: number
}

export function SVGWrapper(props: SVGWrapperProps): JSX.Element {
    const theme = useTheme();
    return (
        <div
            style={{ margin: theme.spacing(2) }}
            {...props}
        >
            <props.children />
        </div>
    )
}
