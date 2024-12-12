import { StarOutlineRounded, StarRounded } from "@mui/icons-material/";
import { Rating } from "@mui/material";

export const StarRating = ({
  value,
  defaultValue,
  max = 5,
  size = "medium",
}: {
  value: number;
  defaultValue?: number;
  max?: number;
  size?: "small" | "medium" | "large";
}) => {
  return (
    <Rating
      size={size}
      sx={{ color: "accent.main" }}
      value={value}
      defaultValue={defaultValue}
      max={max}
      readOnly
      icon={<StarRounded fontSize={size} />}
      emptyIcon={
        <StarOutlineRounded fontSize={size} sx={{ color: "accent.main" }} />
      }
    />
  );
};
