import React, { useCallback, useEffect, useState } from 'react';
import { ExpandMore } from '@mui/icons-material';
import {
  Alert,
  Box,
  Collapse,
  Divider,
  Grid,
  IconButton,
  LinearProgress,
  Skeleton,
  Stack,
  Tooltip,
  Typography,
} from '@mui/material';
import { alpha, useTheme, type Theme } from '@mui/material/styles';
import type { Network } from 'src/types';
import {
  fetchGatewayStatusIfBonded,
  findExplorerNymNodeByIdentity,
  getNodeStatusBaseUrl,
  normalizeExplorerUptimePercent,
  type ExplorerNymNodeRow,
  type GatewayStatusPayload,
} from 'src/api/nodeStatus';
import { probeGroupsForChart, socks5LatencyMs, wgComparisonBars, type ProbeGroupBar } from 'src/api/nodeStatusCharts';

const OPERATOR_INSIGHTS_EXPANDED_KEY = 'nymWallet.bonding.operatorInsightsExpanded';

function readOperatorInsightsExpanded(): boolean {
  try {
    const v = localStorage.getItem(OPERATOR_INSIGHTS_EXPANDED_KEY);
    if (v === null) {
      return true;
    }
    return v === 'true';
  } catch {
    return true;
  }
}

type InsightsCollapseFrameProps = {
  title: string;
  subtitle: string;
  children: React.ReactNode;
};

const InsightsCollapseFrame = ({ title, subtitle, children }: InsightsCollapseFrameProps) => {
  const theme = useTheme();
  const [open, setOpen] = useState(readOperatorInsightsExpanded);

  const toggle = useCallback(() => {
    setOpen((prev) => {
      const next = !prev;
      try {
        localStorage.setItem(OPERATOR_INSIGHTS_EXPANDED_KEY, String(next));
      } catch {
        /* ignore */
      }
      return next;
    });
  }, []);

  return (
    <Box sx={{ width: '100%', mt: { xs: 1, md: 0 } }}>
      <Stack
        direction="row"
        alignItems="flex-start"
        justifyContent="space-between"
        sx={{
          gap: 1,
          cursor: 'pointer',
          userSelect: 'none',
          '&:focus-visible': {
            outline: `2px solid ${theme.palette.primary.main}`,
            outlineOffset: 2,
            borderRadius: 1,
          },
        }}
        onClick={toggle}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            toggle();
          }
        }}
        role="button"
        tabIndex={0}
        aria-expanded={open}
      >
        <Box sx={{ flex: 1, minWidth: 0 }}>
          <Typography variant="h6" component="h2" sx={{ fontWeight: 600 }}>
            {title}
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mt: 0.5 }}>
            {subtitle}
          </Typography>
        </Box>
        <IconButton
          size="small"
          aria-label={open ? 'Collapse operator insights' : 'Expand operator insights'}
          onClick={(e) => {
            e.stopPropagation();
            toggle();
          }}
        >
          <ExpandMore
            sx={{
              transform: open ? 'rotate(180deg)' : 'rotate(0deg)',
              transition: theme.transitions.create('transform', { duration: theme.transitions.duration.shortest }),
            }}
          />
        </IconButton>
      </Stack>
      <Collapse in={open}>
        <Box sx={{ pt: 2.5 }}>{children}</Box>
      </Collapse>
    </Box>
  );
};

const ACCENT = '#8482FD';

export type NodeStatusMetadata = {
  displayMoniker?: string;
  locationLabel?: string;
};

export type NodeOperatorInsightsProps = {
  network?: Network;
  identityKey: string;
  walletUptime?: number;
  onStatusLoaded?: (meta: NodeStatusMetadata) => void;
};

function chartCardSx(theme: Theme) {
  return {
    borderRadius: 2,
    border: `1px solid ${theme.palette.divider}`,
    bgcolor:
      theme.palette.mode === 'dark'
        ? theme.palette.nym.nymWallet.nav.background
        : theme.palette.nym.nymWallet.background.subtle,
    p: 2,
    height: '100%',
  } as const;
}

/** Softer digest surface for gateway (single panel, less contrast than nested cards). */
function gatewayDigestSx(theme: Theme) {
  return {
    borderRadius: 2,
    border: `1px solid ${alpha(theme.palette.divider, 0.85)}`,
    background:
      theme.palette.mode === 'dark'
        ? `linear-gradient(145deg, ${alpha(theme.palette.nym.nymWallet.nav.background, 0.97)} 0%, ${alpha(
            ACCENT,
            0.04,
          )} 100%)`
        : theme.palette.nym.nymWallet.background.subtle,
    p: { xs: 2, sm: 2.5 },
    overflow: 'hidden',
  } as const;
}

const GatewayProbeRow = ({ theme, row, isLast }: { theme: Theme; row: ProbeGroupBar; isLast: boolean }) => {
  const { name, pctPassed, passed, total } = row;
  let barColor: string;
  if (pctPassed >= 100) {
    barColor = ACCENT;
  } else if (pctPassed > 0) {
    barColor = alpha(ACCENT, 0.65);
  } else {
    barColor = alpha(theme.palette.text.disabled, 0.35);
  }
  return (
    <Tooltip title={`${passed} of ${total} checks passed`} arrow placement="top" enterDelay={400}>
      <Box sx={{ width: '100%' }}>
        <Stack
          direction="row"
          alignItems="center"
          spacing={1.5}
          sx={{
            py: 1,
            borderBottom: isLast ? 'none' : `1px solid ${alpha(theme.palette.divider, 0.5)}`,
          }}
        >
          <Typography variant="body2" sx={{ minWidth: 76, color: 'text.secondary', fontWeight: 500 }}>
            {name}
          </Typography>
          <Box sx={{ flex: 1, minWidth: 0 }}>
            <LinearProgress
              variant="determinate"
              value={pctPassed}
              sx={{
                height: 5,
                borderRadius: 2.5,
                bgcolor: alpha(ACCENT, theme.palette.mode === 'dark' ? 0.1 : 0.14),
                '& .MuiLinearProgress-bar': {
                  borderRadius: 2.5,
                  bgcolor: barColor,
                },
              }}
            />
          </Box>
          <Typography
            variant="caption"
            sx={{
              minWidth: 56,
              textAlign: 'right',
              fontVariantNumeric: 'tabular-nums',
              color: 'text.secondary',
            }}
          >
            {pctPassed}%
          </Typography>
        </Stack>
      </Box>
    </Tooltip>
  );
};

export const NodeOperatorInsights: React.FC<NodeOperatorInsightsProps> = ({
  network,
  identityKey,
  walletUptime,
  onStatusLoaded,
}) => {
  const theme = useTheme();
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | undefined>();
  const [gateway, setGateway] = useState<GatewayStatusPayload | undefined>();
  const [explorerRow, setExplorerRow] = useState<ExplorerNymNodeRow | undefined>();
  const [explorerMissing, setExplorerMissing] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const baseUrl = getNodeStatusBaseUrl(network);

    const run = async () => {
      setLoading(true);
      setError(undefined);
      setExplorerMissing(false);
      try {
        const g = await fetchGatewayStatusIfBonded(baseUrl, identityKey);
        if (cancelled) {
          return;
        }
        if (g) {
          setGateway(g);
          setExplorerRow(undefined);
          const loc = g.explorer_pretty_bond?.location;
          const locationLabel =
            loc && (loc.city || loc.two_letter_iso_country_code)
              ? [loc.city, loc.two_letter_iso_country_code].filter(Boolean).join(', ')
              : undefined;
          onStatusLoaded?.({
            displayMoniker: g.description?.moniker,
            locationLabel,
          });
          return;
        }

        setGateway(undefined);
        const row = await findExplorerNymNodeByIdentity(network, baseUrl, identityKey);
        if (cancelled) {
          return;
        }
        if (row) {
          setExplorerRow(row);
          setExplorerMissing(false);
          const geo = row.geoip;
          const locationLabel =
            geo && (geo.city || geo.country) ? [geo.city, geo.country].filter(Boolean).join(', ') : undefined;
          onStatusLoaded?.({
            displayMoniker: row.description?.moniker,
            locationLabel,
          });
        } else {
          setExplorerRow(undefined);
          setExplorerMissing(true);
          onStatusLoaded?.({});
        }
      } catch (e) {
        if (!cancelled) {
          setError(e instanceof Error ? e.message : 'Failed to load node status');
          setGateway(undefined);
          setExplorerRow(undefined);
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    };

    run().catch(() => {
      /* errors surfaced via setError inside run */
    });
    return () => {
      cancelled = true;
    };
  }, [network, identityKey, onStatusLoaded]);

  if (loading) {
    return (
      <Stack spacing={2} sx={{ width: '100%' }}>
        <Skeleton variant="rounded" height={120} sx={{ borderRadius: 2 }} />
        <Skeleton variant="rounded" height={180} sx={{ borderRadius: 2 }} />
      </Stack>
    );
  }

  if (error) {
    return (
      <Alert severity="warning" sx={{ borderRadius: 2 }}>
        {error}
      </Alert>
    );
  }

  if (gateway) {
    const outcome = gateway.last_probe_result?.outcome;
    const probeGroups = probeGroupsForChart(outcome);
    const wgBars = wgComparisonBars(outcome);
    const latency = socks5LatencyMs(outcome);
    const perf = Math.min(100, Math.max(0, gateway.performance));
    const wgKind = wgBars[0]?.kind;
    const socksGroup = probeGroups.find((g) => g.name === 'SOCKS5');

    return (
      <InsightsCollapseFrame
        title="Operator insights"
        subtitle="Quiet snapshot of how the network last probed this gateway. Expand for detail."
      >
        <Box sx={gatewayDigestSx(theme)}>
          <Grid container spacing={2.5} alignItems="flex-start">
            <Grid item xs={12} md={4}>
              <Stack spacing={2}>
                <Box>
                  <Typography
                    variant="caption"
                    color="text.secondary"
                    sx={{ letterSpacing: '0.06em', textTransform: 'uppercase', display: 'block', mb: 0.75 }}
                  >
                    Performance
                  </Typography>
                  <Typography
                    variant="h3"
                    sx={{
                      fontWeight: 600,
                      color: ACCENT,
                      lineHeight: 1.1,
                      fontSize: { xs: '1.65rem', sm: '1.9rem' },
                    }}
                  >
                    {perf}%
                  </Typography>
                  <Typography
                    variant="caption"
                    color="text.secondary"
                    sx={{ display: 'block', mt: 0.75, lineHeight: 1.45 }}
                  >
                    Reward-weight score (0-100) from the status API.
                  </Typography>
                  <LinearProgress
                    variant="determinate"
                    value={perf}
                    sx={{
                      height: 4,
                      borderRadius: 2,
                      mt: 1.25,
                      bgcolor: alpha(ACCENT, 0.14),
                      '& .MuiLinearProgress-bar': { bgcolor: ACCENT, borderRadius: 2 },
                    }}
                  />
                </Box>
                <Divider flexItem sx={{ borderColor: alpha(theme.palette.divider, 0.55) }} />
                <Box>
                  <Typography variant="caption" color="text.secondary" sx={{ display: 'block' }}>
                    Routing score
                  </Typography>
                  <Typography variant="body2" sx={{ mt: 0.35, fontWeight: 500 }}>
                    {gateway.routing_score > 0 ? gateway.routing_score : 'Not published'}
                  </Typography>
                  <Typography
                    variant="caption"
                    color="text.secondary"
                    sx={{ display: 'block', mt: 0.75, lineHeight: 1.45 }}
                  >
                    Distinct from performance. Often 0 until the network publishes it.
                  </Typography>
                </Box>
                {gateway.last_testrun_utc ? (
                  <Typography variant="caption" color="text.secondary" sx={{ fontVariantNumeric: 'tabular-nums' }}>
                    Last probe: {gateway.last_testrun_utc}
                  </Typography>
                ) : null}
              </Stack>
            </Grid>
            <Grid item xs={12} md={8}>
              <Typography
                variant="caption"
                color="text.secondary"
                sx={{ letterSpacing: '0.06em', textTransform: 'uppercase', display: 'block', mb: 0.5 }}
              >
                Categories
              </Typography>
              <Typography variant="body2" color="text.secondary" sx={{ mb: 1.25, lineHeight: 1.5 }}>
                Hover a row for pass counts.
              </Typography>
              {probeGroups.length === 0 ? (
                <Typography variant="body2" color="text.secondary">
                  No probe data yet.
                </Typography>
              ) : (
                <Box>
                  {probeGroups.map((row, i) => (
                    <GatewayProbeRow key={row.name} theme={theme} row={row} isLast={i === probeGroups.length - 1} />
                  ))}
                </Box>
              )}
              {socksGroup !== undefined && socksGroup.pctPassed < 100 ? (
                <Typography
                  variant="caption"
                  color="text.secondary"
                  sx={{ display: 'block', mt: 1.25, lineHeight: 1.5 }}
                >
                  SOCKS5 / NR did not fully pass this run - optional path, depends on topology.
                </Typography>
              ) : null}
              {latency !== undefined ? (
                <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mt: 0.75 }}>
                  SOCKS5 HTTPS latency: {latency} ms
                </Typography>
              ) : null}
            </Grid>
            {wgBars.length > 0 ? (
              <Grid item xs={12}>
                <Divider sx={{ borderColor: alpha(theme.palette.divider, 0.55), my: 0.25 }} />
                <Typography
                  variant="caption"
                  color="text.secondary"
                  sx={{ letterSpacing: '0.06em', textTransform: 'uppercase', display: 'block', mb: 0.75 }}
                >
                  WireGuard
                </Typography>
                <Typography variant="body2" color="text.secondary" sx={{ mb: 1.25, lineHeight: 1.5 }}>
                  {wgKind === 'milliseconds'
                    ? 'Probe download time over the tunnel (ms). Lower reads faster.'
                    : 'ICMP success rate through the tunnel. Higher is better.'}
                </Typography>
                <Stack direction={{ xs: 'column', sm: 'row' }} spacing={1.5}>
                  {wgBars.map((b) => (
                    <Box
                      key={b.name}
                      sx={{
                        flex: 1,
                        borderRadius: 1.5,
                        px: 2,
                        py: 1.5,
                        bgcolor: alpha(theme.palette.text.primary, 0.035),
                        border: `1px solid ${alpha(theme.palette.divider, 0.4)}`,
                      }}
                    >
                      <Typography variant="caption" color="text.secondary">
                        {b.name}
                      </Typography>
                      <Typography variant="h6" sx={{ fontWeight: 600, mt: 0.35, fontVariantNumeric: 'tabular-nums' }}>
                        {b.kind === 'milliseconds' ? `${b.value} ms` : `${b.value}%`}
                      </Typography>
                    </Box>
                  ))}
                </Stack>
              </Grid>
            ) : null}
          </Grid>
        </Box>
      </InsightsCollapseFrame>
    );
  }

  if (explorerRow) {
    const apiUptime = normalizeExplorerUptimePercent(explorerRow.uptime);
    const epochUptimePct =
      walletUptime !== undefined && walletUptime !== null ? normalizeExplorerUptimePercent(walletUptime) : undefined;

    return (
      <InsightsCollapseFrame
        title="Operator insights"
        subtitle="Uptime from the explorer index versus epoch uptime from the chain. Stake, delegators, and saturation stay in the table above."
      >
        <Box sx={chartCardSx(theme)}>
          <Grid container spacing={3}>
            <Grid item xs={12} sm={6}>
              <Typography variant="body2" color="text.secondary" fontWeight={600}>
                Explorer uptime
              </Typography>
              <Typography
                variant="caption"
                color="text.secondary"
                sx={{ display: 'block', mt: 0.5, mb: 1.25, lineHeight: 1.5 }}
              >
                Public index value. The API may send a 0-1 ratio (for example 0.98) - shown here as 0-100%.
              </Typography>
              <LinearProgress
                variant="determinate"
                value={apiUptime}
                sx={{
                  height: 10,
                  borderRadius: 5,
                  bgcolor: alpha(ACCENT, 0.2),
                  '& .MuiLinearProgress-bar': { bgcolor: ACCENT, borderRadius: 5 },
                }}
              />
              <Typography variant="body2" sx={{ fontWeight: 600, mt: 0.75 }}>
                {apiUptime.toFixed(1)}%
              </Typography>
            </Grid>
            <Grid item xs={12} sm={6}>
              <Typography variant="body2" color="text.secondary" fontWeight={600}>
                Epoch uptime (wallet)
              </Typography>
              <Typography
                variant="caption"
                color="text.secondary"
                sx={{ display: 'block', mt: 0.5, mb: 1.25, lineHeight: 1.5 }}
              >
                Estimate from your wallet RPC for the current epoch (same scale as the explorer bar).
              </Typography>
              {epochUptimePct !== undefined ? (
                <>
                  <LinearProgress
                    variant="determinate"
                    value={epochUptimePct}
                    sx={{
                      height: 10,
                      borderRadius: 5,
                      mt: 0,
                      bgcolor: alpha(theme.palette.primary.main, 0.15),
                    }}
                  />
                  <Typography variant="body2" sx={{ fontWeight: 600, mt: 0.75 }}>
                    {epochUptimePct.toFixed(1)}%
                  </Typography>
                </>
              ) : (
                <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
                  Not available from the wallet for this node.
                </Typography>
              )}
            </Grid>
          </Grid>
        </Box>
      </InsightsCollapseFrame>
    );
  }

  if (explorerMissing) {
    return (
      <InsightsCollapseFrame
        title="Operator insights"
        subtitle="Explorer index for your node (identity match). Cached after the first full scan."
      >
        <Alert severity="info" sx={{ borderRadius: 2 }}>
          Your node is not in the explorer listing yet, or the identity does not match the index. New bonds can take
          time to appear. Insights will load automatically once indexed.
        </Alert>
      </InsightsCollapseFrame>
    );
  }

  return null;
};
