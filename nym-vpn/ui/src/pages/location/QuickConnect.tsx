import { useTranslation } from 'react-i18next';
import { QuickConnectCountry } from '../../constants';
interface QuickConnectProps {
  onClick: (name: string, code: string) => void;
}
export default function QuickConnect({ onClick }: QuickConnectProps) {
  const { t } = useTranslation('nodeLocation');
  return (
    <div
      role="presentation"
      onKeyDown={() =>
        onClick(QuickConnectCountry.name, QuickConnectCountry.code)
      }
      className="flex flex-row w-full py-8 cursor-pointer"
      onClick={() =>
        onClick(QuickConnectCountry.name, QuickConnectCountry.code)
      }
    >
      <span className="font-icon px-4 cursor-pointer">bolt</span>
      <div className="cursor-pointer">{`${t('quick-connect')} (${
        QuickConnectCountry.name
      })`}</div>
    </div>
  );
}
