import clsx from 'clsx';
import { InputEvent } from '../../types';

interface SearchProps {
  value: string;
  onChange: (e: InputEvent) => void;
  placeholder: string;
}

export default function SearchBox({
  value,
  onChange,
  placeholder,
}: SearchProps) {
  return (
    <div className="w-full relative flex flex-row items-center px-4 mb-2">
      <input
        type="text"
        id="country_search"
        value={value}
        className={clsx([
          'bg-blanc-nacre dark:bg-baltic-sea focus:outline-none focus:ring-0',
          'w-full flex flex-row justify-between items-center py-3 px-4 pl-11',
          'text-baltic-sea dark:text-mercury-pinkish',
          'placeholder:text-cement-feet placeholder:dark:text-mercury-mist',
          'border-cement-feet dark:border-gun-powder border-2 rounded-lg',
          'relative text-base',
        ])}
        placeholder={placeholder}
        onChange={onChange}
      />
      <span
        className={clsx([
          'font-icon text-2xl absolute left-7',
          'text-baltic-sea dark:text-laughing-jack',
        ])}
      >
        search
      </span>
    </div>
  );
}
