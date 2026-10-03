import assert from 'node:assert/strict';
import test from 'node:test';

import {
  buildSessionPricingPayload,
  SESSION_PRICING_MODELS,
} from '../static/session-utils.js';

const manifestSession = {
  assistant_type: 'omp',
  source_kind: 'manifest',
  source_dir_key: 'local-profile',
  session_id: 'session-123',
};

test('session pricing exposes exactly the supported model identifiers', () => {
  assert.deepEqual(SESSION_PRICING_MODELS, [
    'glm-5.3',
    'deepseek-v4.1-flash',
    'glm-5.3-flash',
  ]);
});

test('session pricing payload preserves full source identity and allows clearing', () => {
  assert.deepEqual(buildSessionPricingPayload(manifestSession, null), {
    session_id: 'session-123',
    source_kind: 'manifest',
    source_dir_key: 'local-profile',
    pricing_model: null,
  });
  assert.deepEqual(buildSessionPricingPayload({
    ...manifestSession,
    source_kind: 'auto',
    source_dir_key: null,
  }, ''), {
    session_id: 'session-123',
    source_kind: 'auto',
    source_dir_key: null,
    pricing_model: null,
  });
  assert.deepEqual(buildSessionPricingPayload({
    ...manifestSession,
    source_kind: 'omp-session',
    source_dir_key: null,
  }, 'glm-5.3'), {
    session_id: 'session-123',
    source_kind: 'omp-session',
    source_dir_key: null,
    pricing_model: 'glm-5.3',
  });
});

test('session pricing payload accepts each supported model and rejects invalid payloads', () => {
  for (const pricingModel of SESSION_PRICING_MODELS) {
    assert.equal(buildSessionPricingPayload(manifestSession, pricingModel).pricing_model, pricingModel);
  }
  assert.deepEqual(buildSessionPricingPayload({
    ...manifestSession,
    assistant_type: 'copilot',
  }, null), {
    session_id: 'session-123',
    source_kind: 'manifest',
    source_dir_key: 'local-profile',
    pricing_model: null,
  });
  assert.throws(() => buildSessionPricingPayload(manifestSession, 'unknown-model'), TypeError);
  assert.throws(() => buildSessionPricingPayload({
    ...manifestSession,
    source_kind: '',
  }, null), TypeError);
  assert.throws(() => buildSessionPricingPayload({
    ...manifestSession,
    source_dir_key: 42,
  }, null), TypeError);
});
