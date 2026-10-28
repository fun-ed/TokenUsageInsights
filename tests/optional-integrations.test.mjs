import assert from 'node:assert/strict';
import test from 'node:test';

import {
  OPTIONAL_INTEGRATIONS_ENABLED_STORAGE_KEY,
  isOptionalIntegrationsEnabled,
  shouldFetchOptionalIntegrationStatus,
  sourceStateLabel,
} from '../static/optional-integrations.js';

test('optional integrations stay disabled unless the user explicitly enables them', () => {
  const storage = { getItem: key => key === OPTIONAL_INTEGRATIONS_ENABLED_STORAGE_KEY ? null : 'unexpected' };
  assert.equal(isOptionalIntegrationsEnabled(storage), false);
  assert.equal(shouldFetchOptionalIntegrationStatus(false, 'integrations'), false);
  assert.equal(shouldFetchOptionalIntegrationStatus(true, 'monthly'), false);
  assert.equal(shouldFetchOptionalIntegrationStatus(true, 'integrations'), true);
});

test('source states have user-readable labels without source data', () => {
  assert.equal(sourceStateLabel('found'), '已驗證');
  assert.equal(sourceStateLabel('invalid'), '格式無效');
  assert.equal(sourceStateLabel('unknown'), '未知');
});
