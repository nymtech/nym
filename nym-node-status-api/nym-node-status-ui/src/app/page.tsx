"use client";

import HighlightedCard from "@/components/HighlightedCard";
import StatCard, { type StatCardProps } from "@/components/StatCard";
import NestedLayoutWithHeader from "@/layouts/NestedLayoutWithHeader";
import Grid from "@mui/material/Grid";

const data: StatCardProps[] = [
  {
    title: "Users",
    value: "14k",
    interval: "Last 30 days",
    trend: "up",
    data: [
      200, 24, 220, 260, 240, 380, 100, 240, 280, 240, 300, 340, 320, 360, 340,
      380, 360, 400, 380, 420, 400, 640, 340, 460, 440, 480, 460, 600, 880, 920,
    ],
  },
  {
    title: "Conversions",
    value: "325",
    interval: "Last 30 days",
    trend: "down",
    data: [
      1640, 1250, 970, 1130, 1050, 900, 720, 1080, 900, 450, 920, 820, 840, 600,
      820, 780, 800, 760, 380, 740, 660, 620, 840, 500, 520, 480, 400, 360, 300,
      220,
    ],
  },
  {
    title: "Event count",
    value: "200k",
    interval: "Last 30 days",
    trend: "neutral",
    data: [
      500, 400, 510, 530, 520, 600, 530, 520, 510, 730, 520, 510, 530, 620, 510,
      530, 520, 410, 530, 520, 610, 530, 520, 610, 530, 420, 510, 430, 520, 510,
    ],
  },
];

export default function Home() {
  return (
    <NestedLayoutWithHeader>
      <Grid
        container
        spacing={2}
        columns={12}
        sx={{ mb: (theme) => theme.spacing(2) }}
      >
        {data.map((card, index) => (
          <Grid key={index} size={{ xs: 12, sm: 6, lg: 3 }}>
            <StatCard {...card} />
          </Grid>
        ))}
        <Grid size={{ xs: 12, sm: 6, lg: 3 }}>
          <HighlightedCard />
        </Grid>
        <Grid size={{ xs: 12, md: 6 }}>
          <p>
            Est sint aliqua elit veniam occaecat aute qui elit laboris qui. Enim
            ut sunt labore adipisicing id aliqua laboris. Proident incididunt ad
            aliquip occaecat officia eu enim aliqua cupidatat reprehenderit
            fugiat in proident. Enim esse nulla cillum nisi sunt proident amet
            sunt occaecat labore non cupidatat anim aute. Est magna sunt cillum
            cupidatat nostrud et aute laboris id commodo velit non nulla.
          </p>
          <p>
            Amet minim nostrud consectetur adipisicing voluptate veniam amet
            deserunt aute. Eiusmod aliqua officia eiusmod quis anim. Officia in
            commodo aute anim id magna ex culpa nostrud.
          </p>
          <p>
            Ut irure culpa minim esse Lorem officia excepteur duis excepteur
            aliquip. Labore anim fugiat reprehenderit ut. Consequat labore sunt
            enim minim amet. Esse incididunt reprehenderit duis quis ut sint. Eu
            occaecat ipsum qui adipisicing ad ipsum consequat deserunt est dolor
            non cupidatat. Ipsum elit incididunt officia anim ut cillum
            cupidatat officia eu eiusmod qui anim tempor quis. Incididunt
            aliquip reprehenderit aliquip esse qui elit ipsum irure veniam
            consectetur officia culpa velit.
          </p>
          <p>
            Eu ad occaecat do fugiat cupidatat aliquip anim occaecat et fugiat
            sunt consectetur eiusmod. Qui adipisicing esse magna esse eu sit
            tempor enim cupidatat nulla sunt. Nulla deserunt eiusmod ullamco
            amet reprehenderit exercitation non est voluptate laboris tempor.
            Est elit dolor occaecat velit dolor cillum commodo qui cillum.
          </p>
          <p>
            Dolore officia eu mollit laborum excepteur quis eu magna aute est
            elit cupidatat reprehenderit nostrud. Nisi ut qui aute eiusmod
            cillum officia reprehenderit ipsum tempor. Culpa proident veniam
            laboris magna laborum aliquip cupidatat laborum fugiat ea minim
            proident velit.
          </p>
        </Grid>
        <Grid size={{ xs: 12, md: 6 }}>
          <p>
            Est sint aliqua elit veniam occaecat aute qui elit laboris qui. Enim
            ut sunt labore adipisicing id aliqua laboris. Proident incididunt ad
            aliquip occaecat officia eu enim aliqua cupidatat reprehenderit
            fugiat in proident. Enim esse nulla cillum nisi sunt proident amet
            sunt occaecat labore non cupidatat anim aute. Est magna sunt cillum
            cupidatat nostrud et aute laboris id commodo velit non nulla.
          </p>
          <p>
            Amet minim nostrud consectetur adipisicing voluptate veniam amet
            deserunt aute. Eiusmod aliqua officia eiusmod quis anim. Officia in
            commodo aute anim id magna ex culpa nostrud.
          </p>
          <p>
            Ut irure culpa minim esse Lorem officia excepteur duis excepteur
            aliquip. Labore anim fugiat reprehenderit ut. Consequat labore sunt
            enim minim amet. Esse incididunt reprehenderit duis quis ut sint. Eu
            occaecat ipsum qui adipisicing ad ipsum consequat deserunt est dolor
            non cupidatat. Ipsum elit incididunt officia anim ut cillum
            cupidatat officia eu eiusmod qui anim tempor quis. Incididunt
            aliquip reprehenderit aliquip esse qui elit ipsum irure veniam
            consectetur officia culpa velit.
          </p>
          <p>
            Eu ad occaecat do fugiat cupidatat aliquip anim occaecat et fugiat
            sunt consectetur eiusmod. Qui adipisicing esse magna esse eu sit
            tempor enim cupidatat nulla sunt. Nulla deserunt eiusmod ullamco
            amet reprehenderit exercitation non est voluptate laboris tempor.
            Est elit dolor occaecat velit dolor cillum commodo qui cillum.
          </p>
          <p>
            Dolore officia eu mollit laborum excepteur quis eu magna aute est
            elit cupidatat reprehenderit nostrud. Nisi ut qui aute eiusmod
            cillum officia reprehenderit ipsum tempor. Culpa proident veniam
            laboris magna laborum aliquip cupidatat laborum fugiat ea minim
            proident velit.
          </p>
        </Grid>
      </Grid>
    </NestedLayoutWithHeader>
  );
}
