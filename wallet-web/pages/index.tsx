import React from 'react';
import Head from 'next/head';
import { Theme, makeStyles, createStyles } from '@material-ui/core/styles';
import SignIn from '../components/SignIn';

const useStyles = makeStyles((theme: Theme) =>
  createStyles({
    root: {
      textAlign: 'center',
      paddingTop: theme.spacing(4),
    },
  })
);

const Home = () => {
  const classes = useStyles({});

  return (
    <React.Fragment>
      <Head>
        <title>Nym Wallet</title>
      </Head>
      <div className={classes.root}>
        <SignIn />
      </div>
    </React.Fragment>
  );
};

export default Home;
