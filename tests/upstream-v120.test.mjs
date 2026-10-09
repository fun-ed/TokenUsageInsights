import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync, mkdtempSync, mkdirSync, writeFileSync, readdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { runInNewContext } from 'node:vm';
import { compareSessionRows, sessionIdentityKey, parentSessionIdentityKey } from '../static/session-utils.js';

const appSource = readFileSync(new URL('../static/app.js', import.meta.url), 'utf8');
const indexSource = readFileSync(new URL('../static/index.html', import.meta.url), 'utf8');

function appFunction(name, globals = {}) {
  const source = appSource.match(new RegExp(`^function ${name}\\([^]*?^}`, 'm'))?.[0];
  assert.ok(source, `missing ${name}`);
  return runInNewContext(`${source}\n${name}`, globals);
}

const formatNumber = appFunction('formatNumber');
const formatCost = appFunction('formatCost', { formatNumber });

test('cost formatting rounds the threshold and groups large dollar amounts', () => {
  for (const [value, expected] of [
    [null, '-'], [undefined, '-'], ['invalid', '-'], [0, '$0.00'],
    [12.345, '$12.35'], [999.994, '$999.99'], [999.999, '$1,000'],
    [1000, '$1,000'], [1213.2, '$1,213'], [12345.67, '$12,346'],
  ]) {
    assert.equal(formatCost(value), expected);
  }
});

for (const period of ['Monthly', 'Yearly']) {
  test(`${period} project table renders five columns, cost and matching empty state`, () => {
    const rows = [];
    const tbody = { innerHTML: '', appendChild: row => rows.push(row) };
    const render = appFunction(`render${period}ProjectsTable`, {
      document: { getElementById: () => tbody, createElement: () => ({ style: {} }) },
      t: key => key, escapeHtml: value => String(value).replaceAll('<', '&lt;'),
      formatToken: value => String(value), formatCost,
    });
    render([{ cwd: '<project>', sessions_count: 2, total_tokens: 100, cost_usd: 1213.2 }]);
    assert.equal(rows.length, 1);
    assert.equal((rows[0].innerHTML.match(/<td\b/g) || []).length, 5);
    assert.match(rows[0].innerHTML, /\$1,213/);
    assert.match(rows[0].innerHTML, /&lt;project>/);
    render([{ cwd: 'no-cost', sessions_count: 1, total_tokens: 0 }]);
    assert.match(rows[1].innerHTML, /\$0\.00/);
    render([]);
    assert.match(tbody.innerHTML, /colspan="5"/);
    const table = indexSource.match(new RegExp(`<table[^]*?<tbody id="${period.toLowerCase()}-projects-body"[^]*?</table>`))?.[0];
    assert.ok(table);
    // Restrict to the closest table, since the expression may include earlier tables.
    const closestTable = table.slice(table.lastIndexOf('<table'));
    assert.equal((closestTable.match(/<th\b/g) || []).length, 5);
    assert.match(closestTable, /data-i18n="col_cost"/);
  });
}

test('orphan Claude subagents keep their badge and do not attach to another source', () => {
  const flatten = appFunction('sortAndGetFlatSessions', {
    compareSessionRows, sessionIdentityKey, parentSessionIdentityKey,
  });
  const rows = flatten([
    { assistant_type: 'claude', source_kind: 'claude-default', source_dir_key: 'default', session_id: 'parent', session_name: 'Main' },
    { assistant_type: 'claude', source_kind: 'claude-profile:work', source_dir_key: 'work', session_id: 'agent-a', parent_session_id: 'parent' },
  ], 'session_id', 'asc');
  const orphan = rows.find(row => row.session_id === 'agent-a');
  assert.equal(orphan.depth, 0);
  assert.equal(orphan.isSubagent, true);
  assert.equal(orphan.parentName, 'parent');
  assert.match(appSource, /s\.depth > 0 \? `<span class="tree-connector"/);
});

test('timeline metadata prefers API values and falls back to the selected subagent', () => {
  const elements = new Map();
  const globals = {
    document: { getElementById: id => {
      if (!elements.has(id)) elements.set(id, { style: {} });
      return elements.get(id);
    } },
    abbreviateHomePath: value => value, formatToken: value => String(value),
    renderReasoningEffortBadge: () => '', t: key => key,
    currentSessionCwd: '/fixture', currentSessionModel: 'model',
    currentSessionAgentNickname: 'lead', currentSessionAgentRole: 'verify',
    currentSessionTotalTokens: 0, currentSessionCacheTokens: 0,
    currentSessionInputTokens: 0, currentSessionOutputTokens: 0,
    currentSessionReasoningTokens: 0,
  };
  const render = appFunction('renderTimeline', globals);
  render({ timeline: [] });
  assert.equal(elements.get('meta-nickname').textContent, 'lead');
  assert.equal(elements.get('meta-role').textContent, 'verify');
  assert.equal(elements.get('drawer-meta-role-container').style.display, 'flex');
  render({ metadata: { agent_nickname: 'api-name', agent_role: 'api-role' }, timeline: [] });
  assert.equal(elements.get('meta-nickname').textContent, 'api-name');
  assert.equal(elements.get('meta-role').textContent, 'api-role');
  globals.currentSessionAgentNickname = '';
  globals.currentSessionAgentRole = '';
  render({ timeline: [] });
  assert.equal(elements.get('drawer-meta-nickname-container').style.display, 'none');
  assert.equal(elements.get('drawer-meta-role-container').style.display, 'none');
});

for (const assistant of ['antigravity', 'copilot']) {
  test(`${assistant} collector uses one UTC date and preserves incremental writes`, () => {
    const home = mkdtempSync(join(tmpdir(), 'tui-utc-'));
    try {
      const bin = join(home, 'bin');
      mkdirSync(bin);
      // Local date is the next day; two separate clock reads would cross UTC midnight.
      const calls = join(home, 'date-calls');
      writeFileSync(join(bin, 'date'), `#!/bin/sh\necho "$*" >> '${calls}'\nif [ "$1" = '-u' ]; then printf '2026-10-08T23:59:59Z\\n'; else printf '2026-10-09\\n'; fi\n`, { mode: 0o755 });
      const script = new URL(`../shell/${assistant}/statusline-token.sh`, import.meta.url);
      const env = { ...process.env, HOME: home, PATH: `${bin}:${process.env.PATH}`, TZ: 'Asia/Taipei' };
      const payload = { session_id: 'utc-test', conversation_id: 'utc-test', model: { id: 'test' }, context_window: { total_input_tokens: 10, total_output_tokens: 2 } };
      const run = () => {
        const result = spawnSync('bash', [script.pathname], { input: JSON.stringify(payload), encoding: 'utf8', env });
        assert.equal(result.status, 0, result.stderr);
      };
      run();
      const usage = join(home, assistant === 'antigravity' ? '.gemini/antigravity-cli' : '.copilot', 'usage');
      assert.deepEqual(readdirSync(usage), ['usage-2026-10-08.jsonl']);
      const readEntries = () => {
        const result = spawnSync('jq', ['-sc', '.', join(usage, 'usage-2026-10-08.jsonl')], { encoding: 'utf8' });
        assert.equal(result.status, 0, result.stderr);
        return JSON.parse(result.stdout);
      };
      let entries = readEntries();
      assert.equal(entries.length, 1);
      assert.equal(entries[0].timestamp, '2026-10-08T23:59:59Z');
      assert.equal(entries[0].delta_tokens.total, 12);
      assert.equal(readFileSync(calls, 'utf8').trim(), '-u +%Y-%m-%dT%H:%M:%SZ');
      run();
      assert.equal(readEntries().length, 1);
      payload.context_window.total_output_tokens = 5;
      run();
      entries = readEntries();
      assert.equal(entries.length, 2);
      assert.equal(entries[1].delta_tokens.total, 3);
      assert.equal(entries[1].turn_no, 2);
    } finally {
      rmSync(home, { recursive: true, force: true });
    }
  });
}
