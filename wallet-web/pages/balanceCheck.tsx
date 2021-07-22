import React, { useContext, useEffect } from "react";
import Paper from "@material-ui/core/Paper";
import Typography from "@material-ui/core/Typography";
import Button from "@material-ui/core/Button";
import RefreshIcon from "@material-ui/icons/Refresh";
import { makeStyles } from "@material-ui/core/styles";
import { useRouter } from "next/router";
import { ValidatorClientContext } from "../contexts/ValidatorClient";
import MainNav from "../components/MainNav";
import Confirmation from "../components/Confirmation";
import NoClientError from "../components/NoClientError";
import { useGetBalance } from "../hooks/useGetBalance";

const useStyles = makeStyles((theme) => ({
  appBar: {
    position: "relative",
  },
  layout: {
    width: "auto",
    marginLeft: theme.spacing(2),
    marginRight: theme.spacing(2),
    [theme.breakpoints.up(600 + theme.spacing(2) * 2)]: {
      width: 600,
      marginLeft: "auto",
      marginRight: "auto",
    },
  },
  paper: {
    marginTop: theme.spacing(3),
    marginBottom: theme.spacing(3),
    padding: theme.spacing(2),
    [theme.breakpoints.up(600 + theme.spacing(3) * 2)]: {
      marginTop: theme.spacing(6),
      marginBottom: theme.spacing(6),
      padding: theme.spacing(3),
    },
  },
  buttons: {
    display: "flex",
    justifyContent: "flex-end",
  },
  button: {
    marginTop: theme.spacing(3),
    marginLeft: theme.spacing(1),
  },
}));

export default function CheckBalance() {
  const classes = useStyles();
  const router = useRouter();

  const { client } = useContext(ValidatorClientContext);
  const { getBalance, isBalanceLoading, balanceCheckError, accountBalance } =
    useGetBalance();

  useEffect(() => {
    const updateBalance = async () => {
      if (client === null) {
        await router.push("/");
      } else {
        await getBalance();
      }
    };
    updateBalance();
  }, [client]);

  const balanceMessage = `Current account balance is ${accountBalance}`;

  return (
    <React.Fragment>
      <MainNav />
      <main className={classes.layout}>
        <Paper className={classes.paper}>
          <Typography component="h1" variant="h4" align="center">
            Check Balance
          </Typography>

          {client === null ? (
            <NoClientError />
          ) : (
            <React.Fragment>
              <Confirmation
                isLoading={isBalanceLoading}
                error={balanceCheckError}
                progressMessage="Checking balance..."
                successMessage={balanceMessage}
                failureMessage="Failed to check the account balance!"
              />
              <div className={classes.buttons}>
                <Button
                  variant="contained"
                  color="primary"
                  type="submit"
                  onClick={getBalance}
                  disabled={isBalanceLoading}
                  className={classes.button}
                  startIcon={<RefreshIcon />}
                >
                  Refresh
                </Button>
              </div>
            </React.Fragment>
          )}
        </Paper>
      </main>
    </React.Fragment>
  );
}
