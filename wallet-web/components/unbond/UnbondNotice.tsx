import {Button, Grid} from "@material-ui/core";
import Typography from "@material-ui/core/Typography";
import React from "react";
import { theme } from "../../lib/theme";
import { makeBasicStyle } from "../../common/helpers";

type unbondNoticeProps = {
    onClick: (event: any) => void
}

export default function UnbondNotice(props: unbondNoticeProps) {
    const classes = makeBasicStyle(theme);

    return (
        <React.Fragment>
            <Grid container spacing={3}>
                <Grid item xs={12}>
                    <Typography gutterBottom>
                        You can only have 1 mixnode or gateway per account. Unbond it by pressing the button below.
                    </Typography>
                </Grid>
            </Grid>
            <div className={classes.buttons}>
                <Button
                    variant="contained"
                    color="primary"
                    type="submit"
                    className={classes.button}
                    onClick={props.onClick}
                >
                    Unbond
                </Button>
            </div>
        </React.Fragment>
    )
}