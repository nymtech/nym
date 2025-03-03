import { StarOutlineRounded, StarRounded } from "@mui/icons-material";
import { Rating } from "@mui/material";

const StarRating = ({
  value,
  defaultValue,
  max = 4,
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

export default StarRating;
