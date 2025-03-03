import type { SxProps } from "@mui/system"; // Asegúrate de importar SxProps si estás usando @mui/system

export type CommonButtonProps = {
  text: string;
  isSuccess?: boolean;
  isDisabled?: boolean;
  isTextButton?: boolean;
  isSmall?: boolean;
  isXtraSmall?: boolean;
  isFullWidth?: boolean;
  isSecondary?: boolean;
  isContrast?: boolean;
  sx?: SxProps;
  isLoading?: boolean;
  buttonId?: string;
  type?: "button" | "submit" | "reset";
  startIcon?: React.ReactNode;
  endIcon?: React.ReactNode;
};

export type ButtonWithClick = CommonButtonProps & {
  handleClick: (() => void) | (() => Promise<void>) | (() => null);
  goToLink?: never;
};

export type ButtonWithLink = CommonButtonProps & {
  goToLink: { path: string; target: "_self" | "_blank" | "_parent" | "_top" };
  handleClick?: never;
};

export type ButtonProps = ButtonWithClick | ButtonWithLink;
