import { Box } from "@mui/material";
import Image from "next/image";
import Link from "next/link";
import explorerLogo from "../public/images/smiley.png";
export const Matrix = () => {
  return (
    <Link
      href={"https://nym.com/explorer"}
      target="_blank"
      rel="noopener noreferrer"
    >
      <Image
        src={explorerLogo}
        alt={"Network Explorer"}
        width={20}
        height={24}
      />
    </Link>
  );
};
