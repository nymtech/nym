import React, { useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { format } from 'date-fns';
import { ExpandMore } from '@mui/icons-material';
import {
  Accordion,
  AccordionDetails,
  AccordionSummary,
  Box,
  Collapse,
  IconButton,
  LinearProgress,
  Stack,
  Typography,
} from '@mui/material';
import { alpha, useTheme } from '@mui/material/styles';
import { Line, LineChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts';
import { AppContext } from 'src/context/main';
import {
  CurrentEpochWithEnd,
  EpochRewardsData,
  fetchCurrentEpoch,
  fetchEpochRewards,
  fetchNymPriceDeduped,
  fetchPacketsAndStaking,
  fetchTotalDelegationsCount,
  formatCompactNumber,
  getNetworkOverviewEndpoints,
  NymTokenomics,
  PacketsAndStakingPoint,
} from 'src/api/networkOverview';

const TRAFFIC_CHART_EXPANDED_KEY = 'nymWallet.networkOverview.trafficChartExpanded';
const SECTION_EXPANDED_KEY = 'nymWallet.networkOverview.sectionExpanded';

function readSectionExpanded(): boolean {
  try {
    const v = localStorage.getItem(SECTION_EXPANDED_KEY);
    if (v === null) {
      return true;
    }
    return v === 'true';
  } catch {
    return true;
  }
}

function readTrafficChartExpanded(): boolean {
  try {
    const v = localStorage.getItem(TRAFFIC_CHART_EXPANDED_KEY);
    if (v === null) {
      return true;
    }
    return v === 'true';
  } catch {
    return true;
  }
}

function epochProgressPercent(epoch: CurrentEpochWithEnd): number {
  const start = new Date(epoch.current_epoch_start).getTime();
  const end = new Date(epoch.current_epoch_end).getTime();
  const now = Date.now();
  if (end <= start) {
    return 0;
  }
  const p = ((now - start) / (end - start)) * 100;
  return Math.min(100, Math.max(0, p));
}

function mixnetTrafficHeadline(data: PacketsAndStakingPoint[]): {
  packets: number;
  volumeLabel: string;
  pctChange: number | null;
} | null {
  if (data.length < 3) {
    return null;
  }
  const todaysData = data[data.length - 2];
  const yesterdaysData = data[data.length - 3];
  const noiseLast24H = todaysData.total_packets_sent + todaysData.total_packets_received;
  const noisePrevious24H = yesterdaysData.total_packets_sent + yesterdaysData.total_packets_received;
  const BYTES_PER_PACKET = (2413 + 386) / 2;
  const totalBytes = noiseLast24H * BYTES_PER_PACKET;
  const units = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
  let size = totalBytes;
  let unitIndex = 0;
  for (; size >= 1024 && unitIndex < units.length - 1; unitIndex += 1) {
    size /= 1024;
  }
  const volumeLabel = `${size.toFixed(2)} ${units[unitIndex]}`;
  let pctChange: number | null = null;
  if (noisePrevious24H > 0) {
    pctChange = Number.parseFloat((((noiseLast24H - noisePrevious24H) / noisePrevious24H) * 100).toFixed(2));
  }
  return { packets: noiseLast24H, volumeLabel, pctChange };
}

export const NetworkOverviewSection: React.FC = () => {
  const theme = useTheme();
  const { network } = useContext(AppContext);
  const endpoints = useMemo(() => getNetworkOverviewEndpoints(network), [network]);

  const [sectionExpanded, setSectionExpanded] = useState(readSectionExpanded);
  const [trafficChartExpanded, setTrafficChartExpanded] = useState(readTrafficChartExpanded);

  const [packets, setPackets] = useState<PacketsAndStakingPoint[] | undefined>();
  const [packetsErr, setPacketsErr] = useState<string | undefined>();

  const [epoch, setEpoch] = useState<CurrentEpochWithEnd | undefined>();
  const [epochErr, setEpochErr] = useState<string | undefined>();

  const [rewards, setRewards] = useState<EpochRewardsData | undefined>();
  const [rewardsErr, setRewardsErr] = useState<string | undefined>();

  const [price, setPrice] = useState<NymTokenomics | undefined>();
  const [priceErr, setPriceErr] = useState<string | undefined>();

  const [delegationsCount, setDelegationsCount] = useState<number | undefined>();
  const [delegationsErr, setDelegationsErr] = useState<string | undefined>();

  const [epochProgress, setEpochProgress] = useState(0);

  const persistSectionExpanded = useCallback((v: boolean) => {
    setSectionExpanded(v);
    try {
      localStorage.setItem(SECTION_EXPANDED_KEY, String(v));
    } catch {
      /* ignore quota / private mode */
    }
  }, []);

  const persistTrafficChartExpanded = useCallback((v: boolean) => {
    setTrafficChartExpanded(v);
    try {
      localStorage.setItem(TRAFFIC_CHART_EXPANDED_KEY, String(v));
    } catch {
      /* ignore quota / private mode */
    }
  }, []);

  useEffect(() => {
    let cancelled = false;

    const load = async () => {
      setPacketsErr(undefined);
      setEpochErr(undefined);
      setRewardsErr(undefined);
      setPriceErr(undefined);
      setDelegationsErr(undefined);

      const pStats = fetchPacketsAndStaking(endpoints.mixnodeStats)
        .then((d) => {
          if (!cancelled) {
            setPackets(d);
          }
        })
        .catch(() => {
          if (!cancelled) {
            setPacketsErr('Could not load mixnet stats');
          }
        });

      const pEpoch = fetchCurrentEpoch(endpoints.epochCurrent)
        .then((d) => {
          if (!cancelled) {
            setEpoch(d);
          }
        })
        .catch(() => {
          if (!cancelled) {
            setEpochErr('Could not load epoch');
          }
        });

      const pRewards = fetchEpochRewards(endpoints.epochRewards)
        .then((d) => {
          if (!cancelled) {
            setRewards(d);
          }
        })
        .catch(() => {
          if (!cancelled) {
            setRewardsErr('Could not load reward params');
          }
        });

      const pPrice = fetchNymPriceDeduped(endpoints.nymPrice)
        .then((d) => {
          if (!cancelled) {
            setPrice(d);
          }
        })
        .catch(() => {
          if (!cancelled) {
            setPriceErr('Could not load price');
          }
        });

      const pDeleg = fetchTotalDelegationsCount(endpoints.observatoryNodesBase)
        .then((n) => {
          if (!cancelled) {
            setDelegationsCount(n);
          }
        })
        .catch(() => {
          if (!cancelled) {
            setDelegationsErr('Could not load delegations count');
          }
        });

      await Promise.all([pStats, pEpoch, pRewards, pPrice, pDeleg]);
    };

    load().catch(() => {
      /* errors handled per-request */
    });
    return () => {
      cancelled = true;
    };
  }, [endpoints]);

  useEffect(() => {
    if (!epoch) {
      setEpochProgress(0);
      return undefined;
    }
    const tick = () => {
      setEpochProgress(epochProgressPercent(epoch));
    };
    tick();
    const id = window.setInterval(tick, 30_000);
    return () => window.clearInterval(id);
  }, [epoch]);

  const trafficHeadline = useMemo(() => (packets ? mixnetTrafficHeadline(packets) : null), [packets]);

  const trafficChartData = useMemo(() => {
    if (!packets?.length) {
      return [];
    }
    return packets
      .slice(0, -1)
      .map((item) => ({
        label: item.date_utc,
        y: item.total_packets_sent + item.total_packets_received,
      }))
      .filter((row) => row.y >= 2_500_000_000);
  }, [packets]);

  const tvlUsd = useMemo(() => {
    if (!rewards || !price || !packets?.length) {
      return undefined;
    }
    const lastTotalStake = packets[packets.length - 1]?.total_stake ?? 0;
    const poolNym = Number.parseFloat(rewards.interval.reward_pool) / 1_000_000;
    const stakeNym = lastTotalStake / 1_000_000;
    return (poolNym + stakeNym) * price.quotes.USD.price;
  }, [rewards, price, packets]);

  const epochTimes = useMemo(() => {
    if (!epoch) {
      return { start: '', end: '' };
    }
    return {
      start: format(new Date(epoch.current_epoch_start), 'HH:mm:ss'),
      end: format(new Date(epoch.current_epoch_end), 'HH:mm:ss'),
    };
  }, [epoch]);

  const cardShell = (children: React.ReactNode) => (
    <Box
      sx={{
        borderRadius: 2,
        border: (t) => `1px solid ${t.palette.divider}`,
        bgcolor: (t) =>
          t.palette.mode === 'dark' ? 'nym.nymWallet.nav.background' : 'nym.nymWallet.background.subtle',
        p: 2,
      }}
    >
      {children}
    </Box>
  );

  const subtle = theme.palette.text.secondary;
  const trafficColor = '#8482FD';

  const mixnetTrafficBody = () => {
    if (packetsErr) {
      return (
        <Typography color="error" variant="body2" sx={{ mt: 1 }}>
          {packetsErr}
        </Typography>
      );
    }
    if (!trafficHeadline) {
      return (
        <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
          Loading…
        </Typography>
      );
    }
    return (
      <>
        <Stack direction="row" alignItems="baseline" gap={1} flexWrap="wrap" sx={{ mt: 0.5 }}>
          <Typography variant="h5" sx={{ fontWeight: 700 }}>
            {formatCompactNumber(trafficHeadline.packets)}
          </Typography>
          <Typography variant="h6" sx={{ color: trafficColor, fontWeight: 600 }}>
            ({trafficHeadline.volumeLabel})
          </Typography>
        </Stack>
        {trafficHeadline.pctChange !== null ? (
          <Typography
            variant="body2"
            sx={{
              mt: 0.5,
              color: trafficHeadline.pctChange >= 0 ? 'success.main' : 'error.main',
              fontWeight: 600,
            }}
          >
            {trafficHeadline.pctChange >= 0 ? '\u2191 ' : '\u2193 '}
            {Math.abs(trafficHeadline.pctChange)}% (24h)
          </Typography>
        ) : null}
      </>
    );
  };

  const epochBody = () => {
    if (epochErr) {
      return (
        <Typography color="error" variant="body2">
          {epochErr}
        </Typography>
      );
    }
    if (!epoch) {
      return (
        <Typography variant="body2" color="text.secondary">
          Loading…
        </Typography>
      );
    }
    return (
      <>
        <Typography variant="h5" sx={{ fontWeight: 700, fontFamily: 'monospace' }}>
          {epoch.current_epoch_id}
        </Typography>
        <LinearProgress
          variant="determinate"
          value={epochProgress}
          sx={{
            mt: 1,
            height: 8,
            borderRadius: 4,
            bgcolor: alpha(theme.palette.primary.main, 0.15),
          }}
        />
        <Stack direction="row" justifyContent="space-between" sx={{ mt: 0.75 }}>
          <Typography variant="caption" color="text.secondary" sx={{ fontFamily: 'monospace' }}>
            Start {epochTimes.start}
          </Typography>
          <Typography variant="caption" color="text.secondary" sx={{ fontFamily: 'monospace' }}>
            End {epochTimes.end}
          </Typography>
        </Stack>
      </>
    );
  };

  const tokenomicsBody = () => {
    if (priceErr || rewardsErr) {
      return (
        <Typography color="error" variant="body2" sx={{ mt: 1 }}>
          {priceErr ?? rewardsErr}
        </Typography>
      );
    }
    if (!price) {
      return (
        <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
          Loading…
        </Typography>
      );
    }
    return (
      <Stack spacing={1.25} sx={{ mt: 1 }}>
        <Stack direction="row" justifyContent="space-between" alignItems="baseline">
          <Typography fontWeight={700}>NYM</Typography>
          <Typography variant="h6" sx={{ fontWeight: 700, fontFamily: 'monospace' }}>
            ${price.quotes.USD.price.toFixed(4)}
          </Typography>
        </Stack>
        <Box sx={{ borderTop: (t) => `1px solid ${t.palette.divider}`, pt: 1 }}>
          <Stack direction="row" justifyContent="space-between">
            <Typography variant="body2" color="text.secondary">
              Market cap
            </Typography>
            <Typography variant="body2" sx={{ fontFamily: 'monospace' }}>
              ${formatCompactNumber(price.quotes.USD.market_cap)}
            </Typography>
          </Stack>
        </Box>
        <Stack direction="row" justifyContent="space-between">
          <Typography variant="body2" color="text.secondary">
            24h vol
          </Typography>
          <Typography variant="body2" sx={{ fontFamily: 'monospace' }}>
            ${formatCompactNumber(price.quotes.USD.volume_24h)}
          </Typography>
        </Stack>
        <Stack direction="row" justifyContent="space-between">
          <Typography variant="body2" color="text.secondary">
            TVL
          </Typography>
          <Typography variant="body2" sx={{ fontFamily: 'monospace' }}>
            {tvlUsd !== undefined ? `$${formatCompactNumber(tvlUsd)}` : '…'}
          </Typography>
        </Stack>
      </Stack>
    );
  };

  const showTrafficChartToggle = trafficChartData.length > 0;

  return (
    <Accordion
      expanded={sectionExpanded}
      onChange={(_, expanded) => persistSectionExpanded(expanded)}
      elevation={0}
      disableGutters
      sx={{
        width: '100%',
        minWidth: 0,
        borderRadius: 2,
        border: (t) => `1px solid ${t.palette.divider}`,
        bgcolor: 'background.paper',
        '&:before': {
          display: 'none',
        },
      }}
    >
      <AccordionSummary
        expandIcon={<ExpandMore sx={{ color: 'text.secondary' }} />}
        aria-controls="network-overview-panel"
        id="network-overview-header"
        sx={{
          px: 2,
          minHeight: 56,
          '& .MuiAccordionSummary-content': {
            my: 1,
          },
        }}
      >
        <Typography variant="h6" component="h2">
          Network overview
        </Typography>
      </AccordionSummary>
      <AccordionDetails id="network-overview-panel" sx={{ px: 2, pb: 2, pt: 0 }}>
        <Box
          sx={{
            display: 'grid',
            gridTemplateColumns: {
              xs: '1fr',
              md: 'repeat(2, minmax(0, 1fr))',
              lg: 'repeat(3, minmax(0, 1fr))',
            },
            gap: 2,
          }}
        >
          {cardShell(
            <>
              <Stack direction="row" alignItems="flex-start" justifyContent="space-between" gap={1}>
                <Typography variant="caption" color="text.secondary" sx={{ letterSpacing: 0.06, fontWeight: 600 }}>
                  Mixnet traffic
                </Typography>
                {showTrafficChartToggle ? (
                  <IconButton
                    size="small"
                    aria-expanded={trafficChartExpanded}
                    aria-label={trafficChartExpanded ? 'Hide mixnet traffic chart' : 'Show mixnet traffic chart'}
                    onClick={() => persistTrafficChartExpanded(!trafficChartExpanded)}
                    sx={{
                      mt: -0.5,
                      mr: -0.5,
                      color: 'text.secondary',
                      '&:hover': { color: 'primary.main' },
                    }}
                  >
                    <ExpandMore
                      sx={{
                        transform: trafficChartExpanded ? 'rotate(180deg)' : 'rotate(0deg)',
                        transition: theme.transitions.create('transform', {
                          duration: theme.transitions.duration.shortest,
                        }),
                      }}
                    />
                  </IconButton>
                ) : null}
              </Stack>
              {mixnetTrafficBody()}
              <Collapse in={trafficChartExpanded && showTrafficChartToggle} timeout="auto" unmountOnExit>
                <Box sx={{ height: 140, mt: 1, width: '100%', minWidth: 0 }}>
                  <ResponsiveContainer width="100%" height="100%">
                    <LineChart data={trafficChartData} margin={{ top: 4, right: 8, left: 0, bottom: 0 }}>
                      <XAxis
                        dataKey="label"
                        tick={{ fontSize: 10, fill: subtle }}
                        tickFormatter={(v) => format(new Date(v), 'MMM d')}
                      />
                      <YAxis
                        tick={{ fontSize: 10, fill: subtle }}
                        width={40}
                        tickFormatter={(v) => formatCompactNumber(Number(v))}
                      />
                      <Tooltip
                        formatter={(value: number) => [formatCompactNumber(value), 'Packets']}
                        labelFormatter={(v) => format(new Date(v), 'PP')}
                      />
                      <Line
                        type="monotone"
                        dataKey="y"
                        stroke={trafficColor}
                        strokeWidth={2}
                        dot={false}
                        isAnimationActive={false}
                      />
                    </LineChart>
                  </ResponsiveContainer>
                </Box>
              </Collapse>
            </>,
          )}

          {cardShell(
            <>
              <Typography variant="caption" color="text.secondary" sx={{ letterSpacing: 0.06, fontWeight: 600 }}>
                Delegations and epoch
              </Typography>
              <Stack spacing={1.5} sx={{ mt: 1 }}>
                <Box>
                  <Typography variant="caption" color="text.secondary">
                    Number of delegations
                  </Typography>
                  {delegationsErr ? (
                    <Typography color="error" variant="body2">
                      {delegationsErr}
                    </Typography>
                  ) : (
                    <Typography variant="h5" sx={{ fontWeight: 700 }}>
                      {delegationsCount !== undefined ? formatCompactNumber(delegationsCount) : '…'}
                    </Typography>
                  )}
                </Box>
                <Box>
                  <Typography variant="caption" color="text.secondary">
                    Current epoch
                  </Typography>
                  {epochBody()}
                </Box>
              </Stack>
            </>,
          )}

          {cardShell(
            <>
              <Typography variant="caption" color="text.secondary" sx={{ letterSpacing: 0.06, fontWeight: 600 }}>
                Tokenomics
              </Typography>
              {tokenomicsBody()}
            </>,
          )}
        </Box>
      </AccordionDetails>
    </Accordion>
  );
};
