/* eslint-disable @typescript-eslint/naming-convention */
import React from 'react';
import { Box, Button, Chip, Stack, Table, TableBody, TableCell, TableRow, Typography } from '@mui/material';
import { FamilyMemberSections, MemberListSectionKey } from 'src/types/families';
import { NymCard } from '../NymCard';
import { ConfirmActionButton } from './ConfirmActionButton';
import { formatExpiry } from './helpers';

export interface MemberListProps {
  sections: FamilyMemberSections;
  nowSecs: number;
  isLoading?: boolean;
  isError?: boolean;
  isBusy?: boolean;
  onKick: (nodeId: number) => void;
  onRefresh: () => void;
}

const SECTION_TITLES: Record<MemberListSectionKey, string> = {
  pending: 'Pending',
  joined: 'Joined',
  rejected: 'Rejected',
  removed: 'Removed',
};

const Section = ({
  sectionKey,
  count,
  children,
}: {
  sectionKey: MemberListSectionKey;
  count: number;
  children: React.ReactNode;
}) => (
  <Box data-testid={`member-section-${sectionKey}`}>
    <Typography variant="subtitle2" sx={{ mb: 1 }}>
      {SECTION_TITLES[sectionKey]} ({count})
    </Typography>
    {count === 0 ? (
      <Typography variant="body2" color="text.secondary" data-testid={`member-section-${sectionKey}-empty`}>
        No {SECTION_TITLES[sectionKey].toLowerCase()} entries.
      </Typography>
    ) : (
      <Table size="small">
        <TableBody>{children}</TableBody>
      </Table>
    )}
  </Box>
);

export const MemberList = ({ sections, nowSecs, isLoading, isError, isBusy, onKick, onRefresh }: MemberListProps) => {
  if (isError) {
    return (
      <NymCard title="Members" data-testid="member-list">
        <Stack spacing={2}>
          <Typography color="error" data-testid="member-list-error">
            Failed to load the member list.
          </Typography>
          <Box>
            <Button variant="outlined" size="small" onClick={onRefresh}>
              Retry
            </Button>
          </Box>
        </Stack>
      </NymCard>
    );
  }

  return (
    <NymCard
      title="Members"
      data-testid="member-list"
      Action={
        <Button variant="text" size="small" onClick={onRefresh} disabled={isLoading} data-testid="member-list-refresh">
          Refresh
        </Button>
      }
    >
      {isLoading ? (
        <Typography variant="body2" color="text.secondary" data-testid="member-list-loading">
          Loading members…
        </Typography>
      ) : (
        <Stack spacing={3}>
          <Section sectionKey="pending" count={sections.pending.length}>
            {sections.pending.map((r) => (
              <TableRow key={`pending-${r.node_id}`} data-testid={`member-pending-${r.node_id}`}>
                <TableCell>{r.node_id}</TableCell>
                <TableCell align="right">
                  {r.expired ? (
                    <Chip size="small" label="Expired" />
                  ) : (
                    <Typography variant="body2">{formatExpiry(r.expires_at, nowSecs)}</Typography>
                  )}
                </TableCell>
              </TableRow>
            ))}
          </Section>

          <Section sectionKey="joined" count={sections.joined.length}>
            {sections.joined.map((r) => (
              <TableRow key={`joined-${r.node_id}`} data-testid={`member-joined-${r.node_id}`}>
                <TableCell>{r.node_id}</TableCell>
                <TableCell align="right">
                  <ConfirmActionButton
                    label="Remove"
                    color="error"
                    title="Remove member?"
                    body={`Remove node ${r.node_id} from the family? This cannot be undone.`}
                    confirmLabel="Remove member"
                    disabled={isBusy}
                    onConfirm={() => onKick(r.node_id)}
                    dataTestid={`member-joined-${r.node_id}-kick`}
                  />
                </TableCell>
              </TableRow>
            ))}
          </Section>

          <Section sectionKey="rejected" count={sections.rejected.length}>
            {sections.rejected.map((r) => (
              <TableRow key={`rejected-${r.node_id}`} data-testid={`member-rejected-${r.node_id}`}>
                <TableCell>{r.node_id}</TableCell>
                <TableCell align="right">
                  <Chip size="small" label="Rejected" />
                </TableCell>
              </TableRow>
            ))}
          </Section>

          <Section sectionKey="removed" count={sections.removed.length}>
            {sections.removed.map((r) => (
              <TableRow key={`removed-${r.node_id}`} data-testid={`member-removed-${r.node_id}`}>
                <TableCell>{r.node_id}</TableCell>
                <TableCell align="right">
                  <Chip size="small" label="Removed" />
                </TableCell>
              </TableRow>
            ))}
          </Section>
        </Stack>
      )}
    </NymCard>
  );
};
