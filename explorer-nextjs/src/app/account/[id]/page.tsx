import { NYM_ACCOUNT_ADDRESS } from "@/app/api/urls";
import ExplorerCard from "@/components/cards/ExplorerCard";
import { ContentLayout } from "@/components/contentLayout/ContentLayout";
import SectionHeading from "@/components/headings/SectionHeading";
import ExplorerListItem from "@/components/list/ListItem";
import ExplorerButtonGroup from "@/components/toggleButton/ToggleButton";
import { Box, Grid2, Stack } from "@mui/material";

export default async function Account({
  params,
}: {
  params: Promise<{ address: string }>;
}) {
  // const address = (await params).address;
  const address = "n17hllefp8rn3ayk23rsgp7agtxkv56mazv3w637";

  const nymAccountAddress = `${NYM_ACCOUNT_ADDRESS}${address}`;
  const accountData = await fetch(nymAccountAddress, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
    next: { revalidate: 60 },
    // refresh event list cache at given interval
  });
  const nymAccountData = await accountData.json();

  if (!nymAccountData) {
    return null;
  }

  console.log("nymAccountData :>> ", nymAccountData);
  return (
    <ContentLayout>
      <Grid2 container columnSpacing={5} rowSpacing={5}>
        <Grid2 size={6}>
          <SectionHeading title="Account Details" />
        </Grid2>
        <Grid2 size={6} justifyContent="flex-end">
          <Box sx={{ display: "flex", justifyContent: "end" }}>
            <ExplorerButtonGroup
              options={[
                { label: "Nym Node", isSelected: false, link: "/nym-node/1" },
                {
                  label: "Account",
                  isSelected: true,
                  link: "/account/1",
                },
              ]}
            />
          </Box>
        </Grid2>
        <Grid2 size={4}>
          <ExplorerCard label="Action" sx={{ height: "100%" }}>
            <div />
          </ExplorerCard>
        </Grid2>
        <Grid2 size={8}>
          <ExplorerCard label="Basic info">
            <Stack gap={1}>
              <ExplorerListItem
                divider
                label="NYM Address"
                value="0x1234567890"
              />
              <ExplorerListItem
                divider
                label="Identity Key"
                value="0x1234567890"
              />
              <ExplorerListItem
                row
                divider
                label="Node bonded"
                value="24/11/2024"
              />
              <ExplorerListItem row divider label="Nr. of stakes" value="56" />
              <ExplorerListItem row label="Self bonded" value="10,000 NYM" />
            </Stack>
          </ExplorerCard>
        </Grid2>
      </Grid2>
    </ContentLayout>
  );
}
