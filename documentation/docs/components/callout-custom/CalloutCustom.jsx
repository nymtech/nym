export const Clt = ({ children, backgroundColor = 'white', borderColor = 'black', pointPosition = 'left', pointOffset = '2rem', pointAlignment = 'end', pointTranslate, pointLength = '2rem', cornerRadius = '0.5rem', borderWidth = '0.2rem', pointWidthMultiplier = 0.5, pointLengthMultiplier = 1, style = {} }) => {


    const center = pointAlignment === 'center'
    const offset = center ? '50%' : pointOffset
    const translate = center ? (pointPosition === 'left' || pointPosition === 'right') ? 'translateY(-50%)' : 'translateX(-50%)' : pointTranslate
    const wMult = Math.min(pointWidthMultiplier, 0.99)
    const lMult = Math.max(pointLengthMultiplier, 1)

    const props = {
        '--pointLength': pointLength,
        '--positionOffset': offset,
        '--bkg-color': backgroundColor,
        '--bdr-color': borderColor,
        '--corner-radius': cornerRadius,
        '--border-width': borderWidth,
        '--pointWidthMultiplier': wMult,
        '--pointLengthMultiplier': lMult,
        ...style
    }



    return (
        <div className={`clt ${pointPosition}`} style={props}>
            <div className={`clt__bubble ${pointPosition}`}>
                <div className="clt__content">
                    {children}
                </div>
            </div>
            <div className={`clt__point-wrapper ${pointPosition} ${pointAlignment}`} style={{ ...(translate && { transform: translate }) }}>
                <div className={`clt__point ${pointPosition}`} />
            </div>
        </div>
    )
}
