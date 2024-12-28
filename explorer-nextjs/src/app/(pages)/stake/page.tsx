import { ContentLayout } from "@/components/contentLayout/ContentLayout";
import SectionHeading from "@/components/headings/SectionHeading";
import OriginalStakeCard from "@/components/staking/OriginalStakeCard";
import OverviewCards from "@/components/staking/OverviewCards";
import StakeTableWithAction from "@/components/staking/StakeTableWithAction";
import TotalRewardsCard from "@/components/staking/TotalRewardsCard";
import TotalStakeCard from "@/components/staking/TotalStakeCard";
import { Grid2 } from "@mui/material";
import SubHeaderRow from "../../../components/staking/SubHeaderRow";

export default async function StakingPage() {
  return (
    <ContentLayout>
      <SectionHeading title="Staking" />
      <SubHeaderRow />
      <OverviewCards />
      <StakeTableWithAction />
    </ContentLayout>
  );
}
