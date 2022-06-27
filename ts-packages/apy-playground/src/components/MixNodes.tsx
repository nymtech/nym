import * as React from 'react';
import CircularProgress from '@mui/material/CircularProgress';
import {
  Checkbox,
  Stack,
  Box,
  IconButton,
  Paper,
  Slider,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
  tableCellClasses,
  TableContainer,
  Typography,
  Link,
  Chip,
} from '@mui/material';
import ArrowDropDownIcon from '@mui/icons-material/ArrowDropDown';
import ArrowDropUpIcon from '@mui/icons-material/ArrowDropUp';
import { Currency } from '@nymproject/react/currency/Currency';
import { CurrencyAmountString } from '@nymproject/react/currency/CurrencyAmount';
import RestartAltIcon from '@mui/icons-material/RestartAlt';
import { useTheme } from '@mui/material/styles';
import CheckCircleOutlineIcon from '@mui/icons-material/CheckCircleOutline';
import PauseCircleOutlineIcon from '@mui/icons-material/PauseCircleOutline';
import { Api, useAppContext } from '../context';
import { toMajorCurrencyFromCoin } from '../utils/coin';
import { round } from '../utils/round';
import {
  MixNodeBondWithDetails,
  RewardEstimation,
  RewardEstimationParamsForSliders,
  RewardEstimationWithAPY,
} from '../context/types';

const NETWORK_EXPLORER_BASE_URL = 'https://explorer.nymtech.net';
const MAJOR_AMOUNT_FOR_CALCS = 1000;

const selectionChanceToProb = (value: string): number => {
  switch (value.toLowerCase()) {
    case 'veryhigh':
      return 0.95;
    case 'high':
      return 0.8;
    case 'moderate':
      return 0.6;
    case 'low':
      return 0.25;
    default:
      return 0.05;
  }
};

const MinorValue: React.FC<{
  value?: number;
  decimals?: number;
}> = ({ value, decimals = 3 }) =>
  // <CurrencyAmountString
  //   majorAmount={value ? round(value / 1_000_000, decimals).toString() : undefined}
  //   sx={{ flexDirection: 'row-reverse' }}
  // />
  value ? <span>{round(value / 1_000_000, decimals)}</span> : <span>-</span>;

const TableCellValue: React.FC<{
  value?: number;
  decimals?: number;
  suffix?: string;
}> = ({ value, suffix, decimals = 0 }) => (
  <TableCell align="right">
    {value ? round(value, decimals) : '-'}
    {suffix && ` ${suffix}`}
  </TableCell>
);

const ResultValue: React.FC<{
  value?: number;
  decimals?: number;
}> = ({ value, decimals = 0 }) => (
  <>
    <TableCell align="right">
      <MinorValue value={value ? value * 24 : undefined} decimals={decimals} />
    </TableCell>
    <TableCell align="right">
      <MinorValue value={value ? value * 24 * 30 : undefined} decimals={decimals} />
    </TableCell>
    <TableCell align="right">
      <MinorValue value={value ? value * 24 * 365 : undefined} decimals={decimals} />
    </TableCell>
  </>
);

const SliderWithValue: React.FC<{
  label: string;
  value?: number;
  min?: number;
  max?: number;
  scaleValue?: number;
  onChange: (value?: number) => void;
  onReset: () => void;
  display: React.ReactNode;
}> = ({ label, value, min, max, onChange, onReset, display, scaleValue = 1 }) => {
  const minScaled = min !== undefined ? min * scaleValue : undefined;
  const maxScaled = max !== undefined ? max * scaleValue : undefined;
  const valueScaled = value !== undefined ? value * scaleValue : undefined;

  console.log({ label, minScaled, maxScaled, valueScaled });

  return (
    <TableRow>
      <TableCell width="20%">{label}</TableCell>
      <TableCell width="30%" align="left">
        <Stack spacing={2} direction="row">
          <Slider
            value={valueScaled}
            min={minScaled}
            max={maxScaled}
            onChange={(_event, newValue) => {
              const scaledNewValue = (newValue as number) / scaleValue;
              console.log({ label, minScaled, maxScaled, valueScaled, scaledNewValue });
              onChange(scaledNewValue);
            }}
          />
          <IconButton>
            <RestartAltIcon opacity={0.15} onClick={onReset} />
          </IconButton>
        </Stack>
      </TableCell>
      <TableCell width="50%">{display}</TableCell>
    </TableRow>
  );
};

export const InclusionProbabilityDisplay: React.FC<{
  isActive?: boolean;
  value: string;
}> = ({ isActive, value }) => (
  <Stack
    direction="row"
    spacing={1}
    color={(theme) =>
      isActive
        ? theme.palette.nym.networkExplorer.mixnodes.status.active
        : theme.palette.nym.networkExplorer.mixnodes.status.standby
    }
  >
    {isActive ? (
      <Box color="inherit">
        <CheckCircleOutlineIcon fontSize="small" color="inherit" />
      </Box>
    ) : (
      <Box color="inherit">
        <PauseCircleOutlineIcon fontSize="small" color="inherit" />
      </Box>
    )}
    <Box color="inherit">{value}</Box>
  </Stack>
);

export const MixNodeRow: React.FC<{ index: number; mixnode: MixNodeBondWithDetails }> = ({ index, mixnode }) => {
  const theme = useTheme();
  const [open, setOpen] = React.useState<boolean>(false);
  const [showRaw, setShowRaw] = React.useState<boolean>(false);
  const ref = React.useRef<NodeJS.Timeout | null>(null);
  const [result, setResult] = React.useState<RewardEstimationWithAPY | undefined>();
  const [defaultResult, setDefaultResult] = React.useState<RewardEstimation | undefined>();

  const defaultParams: RewardEstimationParamsForSliders = {
    pledge_amount: +(Number.parseFloat(mixnode.mixnode_bond.pledge_amount.amount) / 1_000_000),
    uptime: mixnode.uptime,
    total_delegation: +(Number.parseFloat(mixnode.mixnode_bond.total_delegation.amount) / 1_000_000),
    is_active: true,
  };

  const [params, setParams] = React.useState<RewardEstimationParamsForSliders>(defaultParams);

  const handleChange = (prop: string) => (value: any) => {
    setParams((prevState) => ({ ...prevState, [prop]: value }));
  };

  const handleReset = (prop: string) => () =>
    setParams((prevState) => ({ ...prevState, [prop]: (defaultParams as any)[prop] }));

  React.useEffect(() => {
    if (ref.current) {
      clearTimeout(ref.current);
    }
    ref.current = setTimeout(() => calculate(), 250);
  }, [params.is_active, params.pledge_amount, params.uptime, params.total_delegation]);

  const calculate = async () => {
    const res = await Api.computeRewardEstimation(mixnode.mixnode_bond.mix_node.identity_key, {
      ...params,
      total_delegation: Math.floor(params.total_delegation * 1_000_000),
      pledge_amount: Math.floor(params.pledge_amount * 1_000_000),
    });
    const majorAmountToUseInCalcs = MAJOR_AMOUNT_FOR_CALCS;
    const operatorReward = (res.estimated_operator_reward / 1_000_000) * 24; // epoch_reward * 1 epoch_per_hour * 24 hours
    const delegatorsReward = (res.estimated_delegators_reward / 1_000_000) * 24;
    const totalPledge = Number.parseFloat(mixnode.mixnode_bond.pledge_amount.amount) / 1_000_000;
    // const totalDelegations = Number.parseFloat(mixnode.mixnode_bond.total_delegation.amount) / 1_000_000;

    const operatorRewardScaled = majorAmountToUseInCalcs * (operatorReward / params.pledge_amount);
    const delegatorReward = majorAmountToUseInCalcs * (delegatorsReward / params.total_delegation);

    const nodeApy = ((operatorReward + delegatorsReward) / (totalPledge + params.total_delegation)) * 365 * 100;

    const res2: RewardEstimationWithAPY = {
      ...res,
      estimates: {
        majorAmountToUseInCalcs,
        nodeApy,
        operator: {
          apy: (operatorRewardScaled / majorAmountToUseInCalcs) * 365 * 100,
          rewardMajorAmount: {
            daily: operatorRewardScaled,
            monthly: operatorRewardScaled * 30,
            yearly: operatorRewardScaled * 365,
          },
        },
        delegator: {
          apy: (delegatorReward / majorAmountToUseInCalcs) * 365 * 100,
          rewardMajorAmount: {
            daily: delegatorReward,
            monthly: delegatorReward * 30,
            yearly: delegatorReward * 365,
          },
        },
      },
    };
    if (!defaultResult) {
      setDefaultResult(res);
    } else {
      setResult(res2);
    }
  };

  React.useEffect(() => {
    if (open && !result) {
      calculate();
    }
  }, [open, result]);

  const bond = toMajorCurrencyFromCoin(mixnode.mixnode_bond.pledge_amount);
  const totalDelegation = toMajorCurrencyFromCoin(mixnode.mixnode_bond.total_delegation);
  const totalDelegationFloat = Number.parseFloat(totalDelegation?.amount || '1');
  let color;
  // eslint-disable-next-line default-case
  switch (mixnode.status) {
    case 'active':
      color = theme.palette.nym.networkExplorer.mixnodes.status.active;
      break;
    case 'standby':
      color = theme.palette.nym.networkExplorer.mixnodes.status.standby;
      break;
  }
  return (
    <>
      <TableRow>
        <TableCell>
          {open ? (
            <IconButton onClick={() => setOpen(false)}>
              <ArrowDropUpIcon />
            </IconButton>
          ) : (
            <IconButton onClick={() => setOpen(true)}>
              <ArrowDropDownIcon />
            </IconButton>
          )}
          <Chip sx={{ ml: 1 }} label={`${index + 1}`} variant="outlined" />
        </TableCell>
        <TableCell>
          <Link
            href={`${NETWORK_EXPLORER_BASE_URL}/network-components/mixnode/${mixnode.mixnode_bond.mix_node.identity_key}`}
            target="_blank"
          >
            {mixnode.mixnode_bond.mix_node.identity_key.slice(0, 6)}
            ...
            {mixnode.mixnode_bond.mix_node.identity_key.slice(-6)}
          </Link>
        </TableCell>
        <TableCell>
          <Currency majorAmount={bond} showCoinMark coinMarkPrefix hideFractions sx={{ fontSize: 14 }} />
        </TableCell>
        <TableCell>
          <Currency majorAmount={totalDelegation} showCoinMark coinMarkPrefix hideFractions sx={{ fontSize: 14 }} />
        </TableCell>
        <TableCell>
          <Typography color={(theme) => (mixnode.stake_saturation > 1 ? theme.palette.warning.main : undefined)}>
            {round(mixnode.stake_saturation * 100, 1)}%
          </Typography>
        </TableCell>
        <TableCell>
          <Typography fontSize="inherit" color={color}>
            {mixnode.status}
          </Typography>
        </TableCell>
        <TableCell>{round(mixnode.uptime, 0)}%</TableCell>
        <TableCell>{mixnode.mixnode_bond.mix_node.profit_margin_percent}%</TableCell>
        <TableCell>
          {mixnode.inclusion_probability && (
            <InclusionProbabilityDisplay isActive value={mixnode.inclusion_probability.in_active} />
          )}
        </TableCell>
        <TableCell>{round(mixnode.estimated_operator_apy, 0)}%</TableCell>
        <TableCell>
          <Currency
            majorAmount={{
              amount: defaultResult
                ? ((defaultResult.estimated_operator_reward / 1_000_000) * 24 * 365).toString()
                : '',
              denom: 'NYM',
            }}
            showCoinMark
            coinMarkPrefix
            hideFractions
            sx={{ fontSize: 14 }}
          />
        </TableCell>
        <TableCell>{round(mixnode.estimated_delegators_apy, 0)}%</TableCell>
        <TableCell>
          <Currency
            majorAmount={{
              amount: defaultResult
                ? (
                    (MAJOR_AMOUNT_FOR_CALCS * ((defaultResult.estimated_delegators_reward / 1_000_000) * 24 * 365)) /
                    totalDelegationFloat
                  ).toString()
                : '',
              denom: 'NYM',
            }}
            showCoinMark
            coinMarkPrefix
            hideFractions
            sx={{ fontSize: 14 }}
          />
        </TableCell>
        <TableCell>
          {mixnode.inclusion_probability &&
            round(mixnode.estimated_delegators_apy * selectionChanceToProb(mixnode.inclusion_probability.in_active), 0)}
          %
        </TableCell>
        <TableCell>
          <Currency
            majorAmount={{
              amount:
                defaultResult && mixnode.inclusion_probability
                  ? (
                      ((MAJOR_AMOUNT_FOR_CALCS * ((defaultResult.estimated_delegators_reward / 1_000_000) * 24 * 365)) /
                        totalDelegationFloat) *
                      selectionChanceToProb(mixnode.inclusion_probability.in_active)
                    ).toString()
                  : '',
              denom: 'NYM',
            }}
            showCoinMark
            coinMarkPrefix
            hideFractions
            sx={{ fontSize: 14 }}
          />
        </TableCell>
      </TableRow>
      {open && (
        <TableRow>
          <TableCell colSpan={12}>
            <Paper elevation={3} sx={{ px: 2, py: 2 }}>
              <Box>
                <Table size="small">
                  <TableBody>
                    <SliderWithValue
                      label="Pledge"
                      value={params.pledge_amount}
                      min={0}
                      max={Math.max(1_000_000, (defaultParams.pledge_amount || 0) * 2)}
                      // max={Math.max(1_000_000_000_000, (params.pledge_amount || 0) * 1.2)}
                      onChange={handleChange('pledge_amount')}
                      onReset={handleReset('pledge_amount')}
                      display={
                        <Stack direction="row" spacing={2}>
                          <CurrencyAmountString majorAmount={params.pledge_amount?.toString()} hideFractions />
                          <span>nym</span>
                        </Stack>
                      }
                    />
                    <SliderWithValue
                      label="Total delegations"
                      min={0}
                      max={Math.max(1_000_000, (defaultParams.total_delegation || 0) * 2)}
                      value={params.total_delegation}
                      onChange={handleChange('total_delegation')}
                      onReset={handleReset('total_delegation')}
                      display={
                        <Stack direction="row" spacing={2}>
                          <CurrencyAmountString majorAmount={params.total_delegation?.toString()} hideFractions />
                          <span>nym</span>
                        </Stack>
                      }
                    />
                    <SliderWithValue
                      label="Uptime"
                      min={0}
                      max={100}
                      value={params.uptime}
                      onChange={handleChange('uptime')}
                      onReset={handleReset('uptime')}
                      display={<span>{params.uptime}%</span>}
                    />
                    <TableRow>
                      <TableCell width="20%">In active set?</TableCell>
                      <TableCell width="30%" align="left">
                        <Checkbox
                          checked={params.is_active === true}
                          onChange={(_, checked) => {
                            handleChange('is_active')(checked);
                          }}
                        />
                        <IconButton>
                          <RestartAltIcon opacity={0.15} onClick={handleReset('is_active')} />
                        </IconButton>
                      </TableCell>
                      <TableCell width="50%">{params.is_active === undefined ? '-' : `${params.is_active}`}</TableCell>
                    </TableRow>
                    {result && (
                      <>
                        <TableRow>
                          <TableCell colSpan={4}>
                            <Box>
                              <TableContainer>
                                <Table
                                  sx={{
                                    '& .MuiTableRow-root:hover': {
                                      backgroundColor: 'grey.800',
                                    },
                                    [`& .${tableCellClasses.root}`]: {
                                      borderBottom: 'none',
                                    },
                                  }}
                                >
                                  <TableHead>
                                    <TableRow>
                                      <TableCell colSpan={1} />
                                      <TableCell colSpan={5} align="center">
                                        <strong>Total rewards</strong>
                                      </TableCell>
                                      <TableCell colSpan={4} align="center">
                                        <strong>
                                          When {result.estimates.majorAmountToUseInCalcs} NYM is staked,
                                          <br />
                                          estimated rewards in NYM are:
                                        </strong>
                                      </TableCell>
                                      <TableCell />
                                    </TableRow>
                                    <TableRow>
                                      <TableCell />
                                      <TableCell align="right" sx={{ opacity: 0.2 }}>
                                        <strong>Current per day</strong>
                                      </TableCell>
                                      <TableCell align="right">
                                        <strong>Est. per day</strong>
                                      </TableCell>
                                      <TableCell align="right">
                                        <strong>Est. per month</strong>
                                      </TableCell>
                                      <TableCell align="right">
                                        <strong>Est. per year</strong>
                                      </TableCell>
                                      <TableCell />
                                      <TableCell align="right">
                                        <strong>Daily</strong>
                                      </TableCell>
                                      <TableCell align="right">
                                        <strong>Monthly</strong>
                                      </TableCell>
                                      <TableCell align="right">
                                        <strong>Annual</strong>
                                      </TableCell>
                                      <TableCell align="right">
                                        <strong>APY</strong>
                                      </TableCell>
                                    </TableRow>
                                  </TableHead>
                                  <TableBody>
                                    <TableRow>
                                      <TableCell>Total node reward</TableCell>
                                      <TableCell sx={{ opacity: 0.3 }} align="right">
                                        <MinorValue value={defaultResult?.estimated_total_node_reward} />
                                      </TableCell>
                                      <ResultValue value={result.estimated_total_node_reward} />
                                      <TableCell sx={{ opacity: 0.3 }}>nym</TableCell>
                                      <TableCell />
                                      <TableCell />
                                      <TableCell />
                                      <TableCellValue value={result.estimates.nodeApy} decimals={0} suffix="%" />
                                    </TableRow>
                                    <TableRow>
                                      <TableCell>Operator reward</TableCell>
                                      <TableCell sx={{ opacity: 0.3 }} align="right">
                                        <MinorValue value={defaultResult?.estimated_operator_reward} />
                                      </TableCell>
                                      <ResultValue value={result.estimated_operator_reward} />
                                      <TableCell sx={{ opacity: 0.3 }}>nym</TableCell>
                                      <TableCellValue
                                        value={result.estimates.operator.rewardMajorAmount.daily}
                                        decimals={3}
                                      />
                                      <TableCellValue value={result.estimates.operator.rewardMajorAmount.monthly} />
                                      <TableCellValue value={result.estimates.operator.rewardMajorAmount.yearly} />
                                      <TableCellValue value={result.estimates.operator.apy} suffix="%" />
                                    </TableRow>
                                    <TableRow>
                                      <TableCell>All delegators reward</TableCell>
                                      <TableCell sx={{ opacity: 0.3 }} align="right">
                                        <MinorValue value={defaultResult?.estimated_delegators_reward} />
                                      </TableCell>
                                      <ResultValue value={result.estimated_delegators_reward} />
                                      <TableCell sx={{ opacity: 0.3 }}>nym</TableCell>
                                      <TableCellValue
                                        value={result.estimates.delegator.rewardMajorAmount.daily}
                                        decimals={3}
                                      />
                                      <TableCellValue value={result.estimates.delegator.rewardMajorAmount.monthly} />
                                      <TableCellValue value={result.estimates.delegator.rewardMajorAmount.yearly} />
                                      <TableCellValue value={result.estimates.delegator.apy} suffix="%" />
                                    </TableRow>
                                    <TableRow>
                                      <TableCell>Node profit</TableCell>
                                      <TableCell sx={{ opacity: 0.3 }} align="right">
                                        <MinorValue value={defaultResult?.estimated_node_profit} />
                                      </TableCell>
                                      <ResultValue value={result.estimated_node_profit} />
                                      <TableCell sx={{ opacity: 0.3 }}>nym</TableCell>
                                    </TableRow>
                                    <TableRow>
                                      <TableCell>Operator cost</TableCell>
                                      <TableCell sx={{ opacity: 0.3 }} align="right">
                                        <MinorValue value={defaultResult?.estimated_operator_cost} />
                                      </TableCell>
                                      <ResultValue value={result.estimated_operator_cost} />
                                      <TableCell sx={{ opacity: 0.3 }}>nym</TableCell>
                                    </TableRow>
                                  </TableBody>
                                </Table>
                              </TableContainer>
                            </Box>
                            <Box mt={2}>
                              Raw values
                              {showRaw ? (
                                <IconButton onClick={() => setShowRaw(false)}>
                                  <ArrowDropUpIcon />
                                </IconButton>
                              ) : (
                                <IconButton onClick={() => setShowRaw(true)}>
                                  <ArrowDropDownIcon />
                                </IconButton>
                              )}
                            </Box>
                          </TableCell>
                        </TableRow>
                        {showRaw && (
                          <TableRow>
                            <TableCell>Raw Result</TableCell>
                            <TableCell>
                              <pre>
                                {JSON.stringify(
                                  {
                                    result,
                                    mixnode,
                                  },
                                  null,
                                  2,
                                )}
                              </pre>
                            </TableCell>
                          </TableRow>
                        )}
                      </>
                    )}
                  </TableBody>
                </Table>
              </Box>
            </Paper>
          </TableCell>
        </TableRow>
      )}
    </>
  );
};

export const MixNodes: React.FC = () => {
  const { loading, mixnodes, rewardParams } = useAppContext();

  if (loading) {
    return <CircularProgress />;
  }

  return (
    <>
      <TableContainer>
        <Table
          sx={{
            '& .MuiTableRow-root:hover': {
              backgroundColor: 'grey.A700',
            },
          }}
        >
          <TableHead>
            <TableRow>
              <TableCell colSpan={9} />
              <TableCell colSpan={4} align="center" sx={{ background: (theme) => theme.palette.divider }}>
                Maximum achievable values
                <br />
                (when always in the active set)
              </TableCell>
              <TableCell colSpan={2} align="center">
                More realistic values
                <br />
                (scaled by selection probability)
              </TableCell>
            </TableRow>
            <TableRow>
              <TableCell />
              <TableCell>Identity</TableCell>
              <TableCell>Pledge</TableCell>
              <TableCell>Total delegations</TableCell>
              <TableCell>Saturation</TableCell>
              <TableCell>Status</TableCell>
              <TableCell>Uptime</TableCell>
              <TableCell>Profit Margin</TableCell>
              <TableCell>Selection Probability</TableCell>
              <TableCell sx={{ background: (theme) => theme.palette.divider }}>Est. Operator APY</TableCell>
              <TableCell sx={{ background: (theme) => theme.palette.divider }}>Annual operator rewards</TableCell>
              <TableCell sx={{ background: (theme) => theme.palette.divider }}>Est. All Delegators APY</TableCell>
              <TableCell sx={{ background: (theme) => theme.palette.divider }}>
                Annual delegator rewards
                <br />
                for staking {MAJOR_AMOUNT_FOR_CALCS} NYM
              </TableCell>
              <TableCell>Est. All Delegators APY</TableCell>
              <TableCell>
                Annual delegator rewards
                <br />
                for staking {MAJOR_AMOUNT_FOR_CALCS} NYM
              </TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {(mixnodes || []).map((m, i) => (
              <MixNodeRow key={m.mixnode_bond.mix_node.identity_key} index={i} mixnode={m} />
            ))}
          </TableBody>
        </Table>
      </TableContainer>

      <Box mt={6}>
        <h3>Reward Params (for epoch)</h3>
      </Box>
      <Table>
        <TableRow>
          <TableCell>Epoch reward pool</TableCell>
          <TableCell>
            <Currency
              coinMarkPrefix
              showCoinMark
              hideFractions
              majorAmount={
                rewardParams?.epoch_reward_pool
                  ? toMajorCurrencyFromCoin({
                      amount: rewardParams.epoch_reward_pool,
                      denom: 'unym',
                    })
                  : undefined
              }
            />
          </TableCell>
        </TableRow>
        <TableRow>
          <TableCell>Rewarded set size</TableCell>
          <TableCell>{rewardParams?.rewarded_set_size}</TableCell>
        </TableRow>
        <TableRow>
          <TableCell>Active set size</TableCell>
          <TableCell>{rewardParams?.active_set_size}</TableCell>
        </TableRow>
        <TableRow>
          <TableCell>Staking supply</TableCell>
          <TableCell>
            <Currency
              coinMarkPrefix
              showCoinMark
              hideFractions
              majorAmount={
                rewardParams?.staking_supply
                  ? toMajorCurrencyFromCoin({
                      amount: rewardParams.staking_supply,
                      denom: 'unym',
                    })
                  : undefined
              }
            />
          </TableCell>
        </TableRow>
        <TableRow>
          <TableCell>Sybil resistance percent</TableCell>
          <TableCell>{rewardParams?.sybil_resistance_percent}</TableCell>
        </TableRow>
        <TableRow>
          <TableCell>Active set work factor</TableCell>
          <TableCell>{rewardParams?.active_set_work_factor}</TableCell>
        </TableRow>
      </Table>
    </>
  );
};
