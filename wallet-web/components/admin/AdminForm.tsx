import { Button, Grid, InputAdornment } from "@material-ui/core";
import TextField from "@material-ui/core/TextField";
import React from "react";
import { nativeToPrintable, StateParams } from "@nymproject/nym-validator-client";
import { DENOM } from "../../pages/_app";
import { theme } from "../../lib/theme";
import { makeBasicStyle } from "../../common/helpers";

type AdminFormProps = {
    onSubmit: (event: any) => void
    currentParams: StateParams,
}

export default function AdminForm(props: AdminFormProps) {
    const classes = makeBasicStyle(theme);

    return (
        <form onSubmit={props.onSubmit}>
            <Grid container spacing={3}>
                <Grid item xs={12}>
                    <TextField
                        required
                        id="mix_bond"
                        name="mix_bond"
                        label="Minimum Mixnode BondMixnode"
                        defaultValue={nativeToPrintable(props.currentParams.minimum_mixnode_bond)}
                        fullWidth
                        InputProps={{
                            endAdornment:
                                <InputAdornment position="end">{DENOM}</InputAdornment>
                        }}
                    />
                </Grid>
                <Grid item xs={12}>
                    <TextField
                        required
                        id="gateway_bond"
                        name="gateway_bond"
                        label="Minimum Gateway BondMixnode"
                        defaultValue={nativeToPrintable(props.currentParams.minimum_gateway_bond)}
                        fullWidth
                        InputProps={{
                            endAdornment:
                                <InputAdornment position="end">{DENOM}</InputAdornment>
                        }}
                    />
                </Grid>
                <Grid item xs={12}>
                    <TextField
                        required
                        id="mix_reward"
                        name="mix_reward"
                        label="Mixnode BondMixnode Reward Rate"
                        defaultValue={props.currentParams.mixnode_bond_reward_rate}
                        fullWidth
                    />
                </Grid>
                <Grid item xs={12}>
                    <TextField
                        required
                        id="gateway_reward"
                        name="gateway_reward"
                        label="Gateway BondMixnode Reward Rate"
                        defaultValue={props.currentParams.gateway_bond_reward_rate}
                        fullWidth
                    />
                </Grid>
                <Grid item xs={12}>
                    <TextField
                        required
                        id="epoch_length"
                        name="epoch_length"
                        label="Epoch length (in hours)"
                        defaultValue={props.currentParams.epoch_length}
                        fullWidth
                        InputProps={{
                            endAdornment:
                                <InputAdornment position="end">hours</InputAdornment>
                        }}
                    />
                </Grid>
                <Grid item xs={12}>
                    <TextField
                        required
                        id="active_set"
                        name="active_set"
                        label="Mixnode Active Set Size"
                        defaultValue={props.currentParams.mixnode_active_set_size}
                        fullWidth
                    />
                </Grid>
            </Grid>
            <div className={classes.buttons}>
                <Button
                    variant="contained"
                    color="primary"
                    type="submit"
                    className={classes.button}
                >
                    Update Contract
                </Button>
            </div>
        </form>
    )
}
