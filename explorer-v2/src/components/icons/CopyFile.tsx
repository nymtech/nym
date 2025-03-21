import Image from "next/image";
import { icons } from "@/utils/getIconByName";


const CopyFile = ({ className }: { className?: string }) => (
  <Image
    src={icons.copyFile}
    alt="Copyfile"
    width={16}
    height={16}
    className={className}
  />
);

export default CopyFile;
