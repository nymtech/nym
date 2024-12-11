import { Box, Button, ButtonGroup } from "@mui/material";
import { Link } from "../muiLink";

type Options = {
  label: string;
  isSelected: boolean;
  link: string;
}[];

const ExplorerButtonGroup = ({
  size = "small",
  options,
}: {
  size?: "small" | "medium" | "large";
  options: Options;
}) => {
  return (
    <ButtonGroup size={size}>
      {options.map((option) => (
        <Link
          href={option.link}
          key={option.label}
          sx={{ textDecoration: "none" }}
        >
          <Button
            sx={{
              color: option.isSelected
                ? "primary.contrastText"
                : "text.primary",
              "&:hover": {
                bgcolor: option.isSelected
                  ? "primary.main"
                  : "background.paper",
              },
              bgcolor: option.isSelected ? "primary.main" : "background.paper",
            }}
            variant="outlined"
          >
            {option.label}
          </Button>
        </Link>
      ))}
    </ButtonGroup>
  );
};
export default ExplorerButtonGroup;
