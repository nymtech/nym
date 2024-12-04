import { Link } from "@/components/muiLink";
import type { Languages } from "@/i18n";
import UserIcon from "@/public/icons/user.svg";

export const AccountButton = ({ locale }: { locale: Languages }) => {
  return (
    <Link
      href={`/${locale}/account/login`}
      sx={{
        display: "flex",
        justifyContent: "center",
        alignItems: "center",
        aspectRatio: "1",
        height: "30px",
        width: "30px",
        flexShrink: 0,
      }}
    >
      <UserIcon />
    </Link>
  );
};
