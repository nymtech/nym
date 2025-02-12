import { ContentLayout } from "../../../components/contentLayout/ContentLayout";
import SectionHeading from "../../../components/headings/SectionHeading";
import OverviewCards from "../../../components/staking/OverviewCards";
import StakeTableWithAction from "../../../components/staking/StakeTableWithAction";
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
