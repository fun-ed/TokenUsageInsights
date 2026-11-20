import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';

const appSource = readFileSync(new URL('../static/app.js', import.meta.url), 'utf8');
const indexSource = readFileSync(new URL('../static/index.html', import.meta.url), 'utf8');

function appFunction(name, globals = {}) {
  const source = appSource.match(new RegExp(`^(?:async )?function ${name}\\([^]*?^}`, 'm'))?.[0];
  assert.ok(source, `missing ${name}`);
  return runInNewContext(`${source}\n${name}`, globals);
}

const rangeMaxDays = Number(appSource.match(/^const RANGE_MAX_DAYS = (\d+);$/m)?.[1]);
const getUtcDateString = appFunction('getUtcDateString');
const addUtcDays = appFunction('addUtcDays');
const parseUtcDate = appFunction('parseUtcDate', { getUtcDateString });
const isValidRange = appFunction('isValidRange', { parseUtcDate, RANGE_MAX_DAYS: rangeMaxDays });
const getIsoWeekRange = appFunction('getIsoWeekRange', { addUtcDays, getUtcDateString });
const getRecentWeeksRange = appFunction('getRecentWeeksRange', { addUtcDays, getUtcDateString });

test('the period switcher offers four accessible buttons and keeps all date selectors', () => {
  const switcher = indexSource.match(/<div\b[^>]*id="period-switcher"[^>]*>([^]*?)<\/div>/);
  assert.ok(switcher);
  assert.match(switcher[0], /role="group"/);
  const buttons = [...switcher[1].matchAll(/<button\b[^>]*data-period="([^"]+)"[^>]*aria-pressed="([^"]+)"/g)];
  assert.deepEqual(buttons.map(([, period, pressed]) => [period, pressed]), [
    ['daily', 'true'], ['monthly', 'false'], ['yearly', 'false'], ['range', 'false'],
  ]);
  for (const inputId of ['date-select', 'month-select', 'year-select']) {
    const input = indexSource.match(new RegExp(`<[^>]+id="${inputId}"[^>]*>`));
    assert.ok(input, `missing ${inputId}`);
    assert.match(input[0], /width: 100%/);
  }
});

test('ISO week presets stay Monday-to-Sunday across the calendar-year boundary', () => {
  const range = getIsoWeekRange(0, new Date('2021-01-01T12:00:00Z'));
  assert.deepEqual({ ...range }, { start: '2020-12-28', end: '2021-01-03' });
});

test('date ranges accept leap day and at most 366 inclusive days', () => {
  assert.equal(parseUtcDate('2024-02-29')?.toISOString(), '2024-02-29T00:00:00.000Z');
  assert.equal(isValidRange('2024-01-01', '2024-12-31'), true);
  assert.equal(isValidRange('2024-01-01', '2025-01-01'), false);
  assert.equal(parseUtcDate('2025-02-29'), null);
});

test('date ranges reject malformed dates and reversed endpoints', () => {
  assert.equal(isValidRange('2024-02-30', '2024-03-01'), false);
  assert.equal(isValidRange('2024-02-02', '2024-02-01'), false);
  assert.equal(isValidRange('2024-2-01', '2024-02-01'), false);
});

test('recent-week presets include exactly the requested UTC days through today', () => {
  const range = getRecentWeeksRange(2, new Date('2026-04-09T20:00:00Z'));
  assert.deepEqual({ ...range }, { start: '2026-03-27', end: '2026-04-09' });
});

test('overview never opens a session drawer, while a single agent retains the entrypoint', async () => {
  const document = { getElementById() { throw new Error('single-agent drawer reached'); } };
  await appFunction('openSessionTimeline', { isAllAssistantsScope: () => true, document })({});
  await assert.rejects(
    appFunction('openSessionTimeline', { isAllAssistantsScope: () => false, document })({}),
    /single-agent drawer reached/,
  );
  assert.match(appSource, /isAllAssistantsScope\(\) \? 'disabled aria-disabled="true"' : ''/);
});
