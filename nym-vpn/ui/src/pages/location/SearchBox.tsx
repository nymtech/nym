import {Input} from "@mui/base/Input";
import {InputEvent} from "../../types/general.ts";
interface SearchProps {
    /** The text to display inside the button */
    value: string;
    /** Whether the button can be interacted with */
    onChange : (e : InputEvent) => void;
    placeholder : string
}
export default function SearchBox({ value, onChange, placeholder } : SearchProps) {
    return(
        <div className="flex flex-1 items-end">
            <Input
                className="dark:bg-baltic-sea"
                value={value}
                onChange={onChange}
                startAdornment={
                    <div className="m-8 inline-flex justify-center">
                      <span className="font-icon">
                        search
                      </span>
                    </div>
                }
                slotProps={{
                    input: {
                        className:
                            "dark:bg-baltic-sea w-80 text-sm font-sans font-normal leading-5 px-3 py-2 rounded-lg shadow-md shadow-slate-100 dark:shadow-slate-900 focus:shadow-outline-purple dark:focus:shadow-outline-purple focus:shadow-lg border border-solid border-slate-300 hover:border-purple-500 dark:hover:border-purple-500 focus:border-purple-500 dark:focus:border-purple-500 dark:border-slate-600 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-300 focus-visible:outline-0",
                    },
                }}
                aria-label={placeholder}
                placeholder={placeholder}
            />
        </div>
    )
}