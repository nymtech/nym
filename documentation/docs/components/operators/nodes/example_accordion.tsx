"use client";

// Ported from @nextui-org/react (dropped: deprecated, React-18 only) to MUI, which is
// already a dependency and React-19 ready.
import Accordion from "@mui/material/Accordion";
import AccordionSummary from "@mui/material/AccordionSummary";
import AccordionDetails from "@mui/material/AccordionDetails";
import ExpandMoreIcon from "@mui/icons-material/ExpandMore";

export const App = () => {
  const defaultContent =
    "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.";

  return (
    <>
      {["Accordion 1", "Accordion 2", "Accordion 3"].map((title) => (
        <Accordion key={title}>
          <AccordionSummary expandIcon={<ExpandMoreIcon />} aria-label={title}>
            {title}
          </AccordionSummary>
          <AccordionDetails>{defaultContent}</AccordionDetails>
        </Accordion>
      ))}
    </>
  );
};
