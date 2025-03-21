import Image from "next/image";
import Link from "next/link";
import explorerLogo from "../public/images/smiley.png";

export const Explorer = () => {
  return (
    <Link
      href={"https://nym.com/explorer"}
      target="_blank"
      rel="noopener noreferrer"
    >
      <Image
        src={explorerLogo}
        style = {{
          marginRight: "0.6rem"
        }}
        alt={"Network Explorer"}
        width={24}
        height={24}
      />
    </Link>
  );
};
