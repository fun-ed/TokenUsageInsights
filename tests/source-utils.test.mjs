import assert from 'node:assert/strict';
import test from 'node:test';

import { getClaudeSessionSourceBadge, getOmpSessionSourceBadge } from '../static/source-utils.js';

const hex = (value) => Buffer.from(value, 'utf8').toString('hex');

test('OMP default source badge keeps its backward-compatible label', () => {
  assert.equal(
    getOmpSessionSourceBadge('omp-session'),
    '<span class="badge source-badge" title="OMP default profile">Default</span>',
  );
});

test('OMP profile source badge displays the escaped profile name', () => {
  assert.equal(
    getOmpSessionSourceBadge('omp-profile:work'),
    '<span class="badge source-badge" title="OMP profile">work</span>',
  );
  assert.equal(
    getOmpSessionSourceBadge('omp-profile:<img src="x" onerror="alert(1)">'),
    '<span class="badge source-badge" title="OMP profile">&lt;img src=&quot;x&quot; onerror=&quot;alert(1)&quot;&gt;</span>',
  );
});

test('unknown source kinds do not render an OMP badge', () => {
  assert.equal(getOmpSessionSourceBadge('omp-other'), '');
});

test('additional source badges decode the configured root and escape it', () => {
  assert.equal(
    getClaudeSessionSourceBadge(`claude-source:${hex('/Users/me/雲端/laptop/.claude')}`),
    '<span class="badge source-badge" title="Claude Code additional source: /Users/me/雲端/laptop/.claude">laptop/.claude</span>',
  );
  assert.equal(
    getOmpSessionSourceBadge(`omp-source:${hex('/mnt/<b>/omp')}`),
    '<span class="badge source-badge" title="OMP additional source: /mnt/&lt;b&gt;/omp">&lt;b&gt;/omp</span>',
  );
});

test('malformed additional source kinds do not render a badge', () => {
  assert.equal(getClaudeSessionSourceBadge('claude-source:xyz'), '');
  assert.equal(getClaudeSessionSourceBadge('claude-source:abc'), '');
  assert.equal(getOmpSessionSourceBadge('omp-source:'), '');
  assert.equal(getClaudeSessionSourceBadge('claude-profile:work'), '');
});
