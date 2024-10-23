import Accordion from "@mui/material/Accordion";
import AccordionActions from "@mui/material/AccordionActions";
import AccordionSummary from "@mui/material/AccordionSummary";
import AccordionDetails from "@mui/material/AccordionDetails";
import ExpandMoreIcon from "@mui/icons-material/ExpandMore";
import {
  TableContainer,
  Table,
  TableBody,
  TableCell,
  TableRow,
  Paper,
} from "@mui/material";



export const AccordionTemplate = ({ name, children }) => (

  <Accordion>
    <AccordionSummary
            expandIcon={<ExpandMoreIcon />}
            aria-controls="panel1-content"
            id="panel1-header"
            aria-label="<code>nym-node --help</code> command output"
          >
            <strong>{name}</strong>
    </AccordionSummary>
    <AccordionDetails>
     <div>
       {children}
     </div>
    </AccordionDetails>
  </Accordion>
);

