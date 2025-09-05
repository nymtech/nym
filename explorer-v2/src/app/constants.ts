export const TABLET_WIDTH = "(min-width:700px)";

// import auto-picking function
import { fetchRecommendedNodes } from "./lib/recommended";

// export a promise that resolves to number[]
export const RECOMMENDED_NODES = fetchRecommendedNodes();
