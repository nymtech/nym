import { MonoCard } from "../cards/MonoCard";

export const TokenomicsCard = () => {
  const tokenomicsCard = {
    overTitle: "Tokenomics overview",
    titlePrice: {
      price: 1.15,
      upDownLine: {
        percentage: 10,
        numberWentUp: true,
      },
    },
    dataRows: {
      rows: [
        { key: "Market cap", value: "$ 1000000" },
        { key: "24H VOL", value: "$ 1000000" },
      ],
    },
  };
  return <MonoCard {...tokenomicsCard} />;
};
