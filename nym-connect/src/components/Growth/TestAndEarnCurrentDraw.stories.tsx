import * as React from 'react';
import { ComponentMeta } from '@storybook/react';
import { DateTime, Duration } from 'luxon';
import {
  TestAndEarnCurrentDraw,
  TestAndEarnCurrentDrawEntered,
  TestAndEarnCurrentDrawFuture,
} from './TestAndEarnCurrentDraw';
import { NymShipyardTheme } from '../../theme';
import { DrawEntryStatus } from './context/types';
import { testMarkdown } from './context/mocks/TestAndEarnContext';

export default {
  title: 'Growth/TestAndEarn/Components/Cards/Current Draw',
  component: TestAndEarnCurrentDraw,
} as ComponentMeta<typeof TestAndEarnCurrentDraw>;

export const Valid = () => (
  <NymShipyardTheme>
    <TestAndEarnCurrentDraw
      draw={{
        id: '1',
        start_utc: DateTime.now().toISO(),
        end_utc: DateTime.now()
          .plus(Duration.fromMillis(1000 * 3600))
          .toISO(),
        last_modified: DateTime.now().toISO(),
        word_of_the_day: 'words words words',
      }}
    />
  </NymShipyardTheme>
);

export const EnteredMalformedDraw = () => (
  <NymShipyardTheme>
    <TestAndEarnCurrentDrawEntered
      draw={{
        id: '1',
        start_utc: DateTime.now().toISO(),
        end_utc: DateTime.now()
          .plus(Duration.fromMillis(1000 * 3600))
          .toISO(),
        last_modified: DateTime.now().toISO(),
        word_of_the_day: undefined,
        entry: {
          draw_id: '1',
          status: DrawEntryStatus.pending,
          id: 'aaaa',
          timestamp: DateTime.now().toISO(),
        },
      }}
    />
  </NymShipyardTheme>
);

export const EnteredDraw = () => (
  <NymShipyardTheme>
    <TestAndEarnCurrentDrawEntered
      draw={{
        id: '1',
        start_utc: DateTime.now().toISO(),
        end_utc: DateTime.now()
          .plus(Duration.fromMillis(1000 * 3600))
          .toISO(),
        last_modified: DateTime.now().toISO(),
        word_of_the_day: testMarkdown,
        entry: {
          draw_id: '1',
          status: DrawEntryStatus.pending,
          id: 'aaaa',
          timestamp: DateTime.now().toISO(),
        },
      }}
    />
  </NymShipyardTheme>
);

export const Future = () => (
  <NymShipyardTheme>
    <TestAndEarnCurrentDrawFuture
      draw={{
        id: '1',
        start_utc: DateTime.now()
          .plus(Duration.fromMillis(1000 * 3600))
          .toISO(),
        end_utc: DateTime.now()
          .plus(Duration.fromMillis(1000 * 3600 * 2))
          .toISO(),
        last_modified: DateTime.now().toISO(),
        word_of_the_day: 'words words words',
      }}
    />
  </NymShipyardTheme>
);
