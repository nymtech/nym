import * as React from 'react';

type WrapperProps = {
    mode: string
}

export const OverviewSVG = ({ mode }: WrapperProps) => {
    const [ color, setColor ] = React.useState<string>('#000000');

    React.useEffect(() => {
        if (mode === 'dark') {
            setColor('#FFFFFF');
        } else {
            setColor('#000000');
        }
    }, [mode])
    return (
        <>
            <svg width="25" height="25" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                <path d="M1.4 21.6H22.6" stroke={color} stroke-miterlimit="10" stroke-linecap="round"/>
                <path d="M14.1 2.40002H9.9V21.5H14.1V2.40002Z" stroke={color} stroke-miterlimit="10" stroke-linecap="round"/>
                <path d="M20.8 6.59998H16.6V21.5H20.8V6.59998Z" stroke={color} stroke-miterlimit="10" stroke-linecap="round"/>
                <path d="M7.4 11.8H3.2V21.6H7.4V11.8Z" stroke={color} stroke-miterlimit="10" stroke-linecap="round"/>
            </svg>

        </>
    )
}