import { MonoCard } from "../cards/MonoCard";

export const RewardsCard = () => {
  const rewardsCard = {
    overTitle: "Operator rewards this month",
    title: "198.841720 NYM",
  };
  return <MonoCard {...rewardsCard} />;
};
