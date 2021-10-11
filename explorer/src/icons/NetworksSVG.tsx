import * as React from 'react';

type WrapperProps = {
    mode: string
}

export const NetworkComponentsSVG = ({ mode }: WrapperProps) => {
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
                <path d="M17.2 10.5V4.40002L12 1.40002L6.8 4.40002V10.5L12 13.5L17.2 10.5Z" stroke={color} stroke-miterlimit="10"/>
                <path d="M12 19.6V13.5L6.8 10.5L1.5 13.5V19.6L6.8 22.6L12 19.6Z" stroke={color} stroke-miterlimit="10"/>
                <path d="M22.5 19.6V13.5L17.2 10.5L12 13.5V19.6L17.2 22.6L22.5 19.6Z" stroke={color} stroke-miterlimit="10"/>
            </svg>

        </>
    )
}