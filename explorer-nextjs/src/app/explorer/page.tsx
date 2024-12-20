import CardSkeleton from "@/components/cards/Skeleton";
import { ContentLayout } from "@/components/contentLayout/ContentLayout";
import SectionHeading from "@/components/headings/SectionHeading";
import NodeTableWithAction from "@/components/nodeTable/NodeTableWithAction";
import { Wrapper } from "@/components/wrapper";
import { Suspense } from "react";

export default function ExplorerPage() {
  return (
    <ContentLayout>
      <Wrapper>
        <SectionHeading title="Explorer" />
        <Suspense fallback={<CardSkeleton sx={{ mt: 5 }} />}>
          <NodeTableWithAction />
        </Suspense>
      </Wrapper>
    </ContentLayout>
  );
}
