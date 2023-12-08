import { QuickConnectCountry } from '../../constants';
interface QuickConnectProps {
  onClick: (name: string, code: string) => void;
}
export default function QuickConnect({ onClick }: QuickConnectProps) {
  return (
    <div
      role="presentation"
      onKeyDown={() =>
        onClick(QuickConnectCountry.name, QuickConnectCountry.code)
      }
      className="flex px-5 flex-row w-full py-8 cursor-pointer"
      onClick={() =>
        onClick(QuickConnectCountry.name, QuickConnectCountry.code)
      }
    >
      <span className="font-icon px-4 cursor-pointer">bolt</span>
      <div className="cursor-pointer">{QuickConnectCountry.name}</div>
    </div>
  );
}
