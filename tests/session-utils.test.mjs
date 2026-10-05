import assert from 'node:assert/strict';
import test from 'node:test';

import {
  compareSessionRows,
  filterEntriesBySessionIdentity,
  formatSessionModelDisplay,
  matchesSessionIdentity,
  parentSessionIdentityKey,
  sessionIdentityKey,
} from '../static/session-utils.js';

const mixedTimestampSessions = [
  {
    session_id: 'sql-late',
    timestamp: '2026-07-26 10:00:00',
  },
  {
    session_id: 'iso-early',
    timestamp: '2026-07-26T09:00:00Z',
  },
  {
    session_id: 'offset-middle',
    timestamp: '2026-07-26T17:30:00+08:00',
  },
];

test('session timestamps sort chronologically across supported formats', () => {
  const ascending = [...mixedTimestampSessions]
    .sort((a, b) => compareSessionRows(a, b, 'timestamp', 'asc'));
  const descending = [...mixedTimestampSessions]
    .sort((a, b) => compareSessionRows(a, b, 'timestamp', 'desc'));

  assert.deepEqual(
    ascending.map(session => session.session_id),
    ['iso-early', 'offset-middle', 'sql-late'],
  );
  assert.deepEqual(
    descending.map(session => session.session_id),
    ['sql-late', 'offset-middle', 'iso-early'],
  );
});

test('session sorting preserves numeric and string column behavior', () => {
  const sessions = [
    { session_id: 'b', total_tokens: 2 },
    { session_id: 'a', total_tokens: 1 },
  ];

  assert.deepEqual(
    [...sessions]
      .sort((a, b) => compareSessionRows(a, b, 'total_tokens', 'desc'))
      .map(session => session.session_id),
    ['b', 'a'],
  );
  assert.deepEqual(
    [...sessions]
      .sort((a, b) => compareSessionRows(a, b, 'session_id', 'asc'))
      .map(session => session.session_id),
    ['a', 'b'],
  );
});

test('session identity includes the source directory key', () => {
  const session = {
    session_id: 'shared',
    assistant_type: 'copilot',
    source_kind: 'copilot-app',
    source_dir_key: 'aa',
  };

  assert.equal(matchesSessionIdentity(session, { ...session }), true);
  assert.equal(
    matchesSessionIdentity(session, { ...session, source_dir_key: 'bb' }),
    false,
  );
  assert.notEqual(
    sessionIdentityKey(session),
    sessionIdentityKey({ ...session, source_kind: 'copilot-cli', source_dir_key: null }),
  );
});

test('parent session identity remains scoped to the child source', () => {
  const child = {
    session_id: 'parent__agent',
    parent_session_id: 'parent',
    assistant_type: 'copilot',
    source_kind: 'copilot-app',
    source_dir_key: 'aa',
  };

  assert.equal(
    parentSessionIdentityKey(child),
    sessionIdentityKey({ ...child, session_id: 'parent' }),
  );
  assert.notEqual(
    parentSessionIdentityKey(child),
    sessionIdentityKey({
      ...child,
      session_id: 'parent',
      source_dir_key: 'bb',
    }),
  );
});

test('raw usage filtering preserves the full session source identity', () => {
  const sessions = [{
    assistant_type: 'copilot',
    source_kind: 'copilot-app',
    source_dir_key: 'aa',
    session_id: 'shared-session',
  }];
  const entries = [
    { ...sessions[0], total_tokens: 100 },
    { ...sessions[0], source_dir_key: 'bb', total_tokens: 300 },
    { ...sessions[0], assistant_type: 'codex', total_tokens: 500 },
  ];

  assert.deepEqual(filterEntriesBySessionIdentity(entries, sessions), [entries[0]]);
});

test('OMP model display appends reasoning effort after the provider and model', () => {
  assert.equal(
    formatSessionModelDisplay('claude/opus-5-5', 'high', 'omp'),
    'claude/opus-5-5:high',
  );
  assert.equal(
    formatSessionModelDisplay('claude/opus-5-5:cloud', 'high', 'omp'),
    'claude/opus-5-5:cloud:high',
  );
  assert.equal(
    formatSessionModelDisplay('claude/opus-5-5:high', 'high', 'omp'),
    'claude/opus-5-5:high',
  );
});

test('OMP model display preserves unknown models and blank effort labels', () => {
  assert.equal(formatSessionModelDisplay('', 'high', 'omp'), '');
  assert.equal(
    formatSessionModelDisplay('unknown-provider/unknown-model', '', 'omp'),
    'unknown-provider/unknown-model',
  );
  assert.equal(
    formatSessionModelDisplay('unknown-provider/unknown-model', 'medium', 'omp'),
    'unknown-provider/unknown-model:medium',
  );
});

test('non-OMP model labels stay unchanged and the formatter does not escape HTML', () => {
  assert.equal(
    formatSessionModelDisplay('claude/<opus-5-5>', '<high>', 'claude'),
    'claude/<opus-5-5>',
  );
  assert.equal(
    formatSessionModelDisplay('claude/<opus-5-5>', '<high>', 'omp'),
    'claude/<opus-5-5>:<high>',
  );
});
