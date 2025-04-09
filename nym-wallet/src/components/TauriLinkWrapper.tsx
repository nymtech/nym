import React from 'react';
import { Link, LinkProps } from '@nymproject/react/link/Link';
import { openUrl } from '@tauri-apps/plugin-opener';

export const TauriLink: React.FC<LinkProps & any> = (props) => {
  const { href, onClick, ...restProps } = props;

  const handleClick = async (event: React.MouseEvent<HTMLAnchorElement>) => {
    if (onClick) {
      onClick(event);
    }

    if (href && (href.startsWith('http://') || href.startsWith('https://'))) {
      event.preventDefault();
      console.log('Opening link in browser:', href);
      await openUrl(href);
    }
  };

  return <Link href={href} onClick={handleClick} {...restProps} />;
};