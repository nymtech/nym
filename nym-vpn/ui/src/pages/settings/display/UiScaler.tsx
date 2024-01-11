import { ChangeEvent, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api';
import clsx from 'clsx';
import { useTranslation } from 'react-i18next';
import { useMainDispatch, useMainState } from '../../../contexts';
import { CmdError, StateDispatch } from '../../../types';

function UiScaler() {
  const [slideValue, setSlideValue] = useState(12);
  const dispatch = useMainDispatch() as StateDispatch;
  const { rootFontSize } = useMainState();
  const { t } = useTranslation('settings');

  useEffect(() => {
    setSlideValue(rootFontSize);
  }, [rootFontSize]);

  const handleChange = (e: ChangeEvent<HTMLInputElement>) => {
    setSlideValue(parseInt(e.target.value));
    dispatch({ type: 'set-root-font-size', size: slideValue });
  };

  const setNewFontSize = () => {
    document.documentElement.style.fontSize = `${slideValue}px`;
    dispatch({ type: 'set-root-font-size', size: slideValue });
    invoke('set_root_font_size', { size: slideValue }).catch((e: CmdError) => {
      console.warn(`backend error: ${e.source} - ${e.message}`);
    });
  };

  return (
    <div
      className={clsx([
        'flex flex-row justify-between items-center gap-10',
        'bg-white dark:bg-baltic-sea-jaguar',
        'px-6 py-4 rounded-lg',
      ])}
    >
      <p className="text-base text-baltic-sea dark:text-mercury-pinkish flex-nowrap select-none">
        {`${t('zoom-level')}: ${slideValue}`}
      </p>
      <input
        type="range"
        min="8"
        max="20"
        value={slideValue}
        onChange={handleChange}
        onMouseUp={setNewFontSize}
        onKeyUp={setNewFontSize}
        className="range flex flex-1 accent-melon cursor-pointer"
      />
    </div>
  );
}

export default UiScaler;
