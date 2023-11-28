import React from 'react';
import clsx from 'clsx';
import { useMainState } from '../contexts';

export default function ThemeSetter({
  children,
}: {
  children: React.ReactNode;
}) {
  const { uiMode } = useMainState();

  return (
    <div className={clsx([uiMode === 'Dark' && 'dark', 'h-full'])}>
      {children}
    </div>
  );
}
