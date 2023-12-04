import { useTranslation } from 'react-i18next';
import { quickConnectCountry } from '../../constants.ts';
interface QuickConnectProps {
  onClick: (name: string, code: string) => void;
}
export default function QuickSelect({ onClick }: QuickConnectProps) {
  const { t } = useTranslation('nodeLocation');
  return (
    <div
      className="flex flex-row w-full py-8 cursor-pointer"
      onClick={() =>
        onClick(quickConnectCountry.name, quickConnectCountry.code)
      }
    >
      <span className="font-icon px-4 cursor-pointer">bolt</span>
      <div className="cursor-pointer">{`${t('quick-select')} (${
        quickConnectCountry.name
      })`}</div>
    </div>
  );
}
