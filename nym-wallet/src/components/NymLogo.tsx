import React from 'react'
import Logo from '../images/logo-background.svg'

const imgSize = {
  ['small']: 40,
  ['medium']: 80,
  ['large']: 120,
}

export const NymLogo = ({ size = 'medium' }: { size?: 'small' | 'medium' | 'large' }) => <Logo width={imgSize[size]} />
