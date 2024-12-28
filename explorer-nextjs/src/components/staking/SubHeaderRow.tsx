import NextEpochTime from "@/components/epochtime/EpochTime";
import Grid2 from "@mui/material/Grid2";
import SubHeaderRowActions from "./SubHeaderRowActions";

const SubHeaderRow = () => {
  return (
    <Grid2 container spacing={3} alignItems={"center"}>
      <Grid2 size={{ xs: 12, sm: "grow" }}>
        <NextEpochTime />
      </Grid2>
      <Grid2 size={{ xs: 12, sm: "auto" }}>
        <SubHeaderRowActions />
      </Grid2>
    </Grid2>
  );
};

export default SubHeaderRow;
