import assert from 'node:assert/strict';
import test from 'node:test';

import { getOmpSessionSourceBadge } from '../static/source-utils.js';

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
