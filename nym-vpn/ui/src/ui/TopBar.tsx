import { useEffect, useMemo, useState } from 'react';
import { useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import clsx from 'clsx';
import { useMainState } from '../contexts';
import { AppName } from '../constants';

export type Props = {
  children?: React.ReactNode;
};

type Routes = '' | 'settings' | 'entry-node-location' | 'exit-node-location';
type RoutePaths = `/${Routes}`;

type NavLocation = {
  title: string;
  leftIcon?: React.ReactNode;
  handleLeftNav?: () => void;
  rightIcon?: React.ReactNode;
  handleRightNav?: () => void;
};

type NavBarData = {
  [key in RoutePaths]: NavLocation;
};

export default function TopBar() {
  const state = useMainState();
  const location = useLocation();
  const { t } = useTranslation();

  const [currentNavLocation, setCurrentNavLocation] = useState<NavLocation>({
    title: AppName,
    rightIcon: 'settings',
    handleRightNav: () => {},
  });

  const navBarData = useMemo<NavBarData>(() => {
    return {
      '/': {
        title: AppName,
        rightIcon: 'settings',
        handleRightNav: () => {},
      },
      '/settings': {
        title: t('settings'),
        leftIcon: 'back',
        handleLeftNav: () => {},
      },
      '/entry-node-location': {
        title: t('first-hop-selection'),
        leftIcon: 'back',
        handleLeftNav: () => {},
      },
      '/exit-node-location': {
        title: t('last-hop-selection'),
        leftIcon: 'back',
        handleLeftNav: () => {},
      },
    };
  }, [t]);

  useEffect(() => {
    setCurrentNavLocation(navBarData[location.pathname as RoutePaths]);
  }, [location.pathname, navBarData]);

  return (
    <nav className="flex dark:bg-baltic-sea-jaguar">
      {currentNavLocation?.leftIcon && (
        <button className={clsx([])}>
          <span className={clsx([])}>O</span>
        </button>
      )}
      {currentNavLocation.title}
      {currentNavLocation?.rightIcon && (
        <button className={clsx([])}>
          <span className={clsx([])}>O</span>
        </button>
      )}
    </nav>
  );
}
