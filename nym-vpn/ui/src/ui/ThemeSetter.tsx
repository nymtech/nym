import React from 'react';
import clsx from 'clsx';
import { useMainState } from '../contexts';

export default function ThemeSetter({
  children,
}: {
  children: React.ReactNode;
}) {
  const { uiTheme } = useMainState();

  return (
    <div className={clsx([uiTheme === 'Dark' && 'dark', 'h-full'])}>
      {children}
    </div>
  );
}
