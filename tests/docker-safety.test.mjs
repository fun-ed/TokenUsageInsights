import assert from 'node:assert/strict';
import { chmodSync, existsSync, mkdtempSync, mkdirSync, readFileSync, readdirSync, rmdirSync, unlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import test from 'node:test';

const scriptPath = fileURLToPath(new URL('../docker/tui-docker.sh', import.meta.url));
const dockerfilePath = fileURLToPath(new URL('../docker/Dockerfile', import.meta.url));

const mockDocker = `#!/bin/bash
printf '%s\\n' "$*" >> "$MOCK_DOCKER_LOG"
case "$1" in
  info) exit 0 ;;
  container)
    [[ "$2" == inspect ]] || exit 1
    [[ "\${MOCK_CONTAINER:-absent}" != absent ]] || exit 1
    [[ "$3" == -f ]] || exit 0
    case "$4" in
      *State.Running*) printf '%s\\n' "\${MOCK_RUNNING:-true}" ;;
      *managed-by*) printf '%s\\n' "\${MOCK_CONTAINER_OWNER:-fun-ed/TokenUsageInsights}" ;;
      *\.name*) printf '%s\\n' "\${MOCK_CONTAINER_NAME:-alice}" ;;
      *\.volume*) printf '%s\\n' "\${MOCK_CONTAINER_VOLUME:-tui-alice-data}" ;;
      *\.port*) printf '%s\\n' "\${MOCK_PORT:-3100}" ;;
      *) exit 1 ;;
    esac
    ;;
  volume)
    [[ "$2" == inspect ]] || exit 1
    [[ "\${MOCK_VOLUME:-absent}" != absent ]] || exit 1
    [[ "$3" == -f ]] || exit 0
    case "$4" in
      *managed-by*) printf '%s\\n' "\${MOCK_VOLUME_OWNER:-fun-ed/TokenUsageInsights}" ;;
      *\.name*) printf '%s\\n' "\${MOCK_VOLUME_NAME:-alice}" ;;
      *) exit 1 ;;
    esac
    ;;
  exec)
    for argument in "$@"; do last_argument="$argument"; done
    printf 'simulated child diagnostic: %s\\n' "$last_argument" >&2
    exit "\${MOCK_EXEC_STATUS:-23}"
    ;;
  *) exit 0 ;;
esac
`;

function removeTempDirectory(directory) {
  const binDirectory = join(directory, 'bin');
  for (const entry of readdirSync(binDirectory)) unlinkSync(join(binDirectory, entry));
  rmdirSync(binDirectory);
  for (const entry of readdirSync(directory)) unlinkSync(join(directory, entry));
  rmdirSync(directory);
}

function runWithMockDocker(t, args, options = {}) {
  const directory = mkdtempSync(join(tmpdir(), 'tui-docker-test-'));
  const binDirectory = join(directory, 'bin');
  mkdirSync(binDirectory);
  const dockerPath = join(binDirectory, 'docker');
  const logPath = join(directory, 'docker.log');
  writeFileSync(dockerPath, mockDocker, { mode: 0o755 });
  chmodSync(dockerPath, 0o755);
  t.after(() => removeTempDirectory(directory));

  const result = spawnSync('bash', [scriptPath, ...args], {
    encoding: 'utf8',
    env: {
      ...process.env,
      PATH: `${binDirectory}:${process.env.PATH}`,
      MOCK_DOCKER_LOG: logPath,
      ...options,
    },
  });
  assert.ifError(result.error);
  return { result, calls: readFileSync(logPath, 'utf8').trim().split('\n') };
}

test('rm refuses a same-named container without the fork ownership labels', t => {
  const { result, calls } = runWithMockDocker(t, ['rm', 'alice', '--purge'], {
    MOCK_CONTAINER: 'foreign',
    MOCK_CONTAINER_OWNER: 'someone-else',
    MOCK_VOLUME: 'owned',
  });

  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /不屬於此管理工具/);
  assert.ok(!calls.some(call => /^(rm |volume rm|start |stop |exec |run )/.test(call)), calls.join('\n'));
});
test('rm checks volume ownership before removing its same-named container', t => {
  const { result, calls } = runWithMockDocker(t, ['rm', 'alice', '--purge'], {
    MOCK_CONTAINER: 'owned',
    MOCK_VOLUME: 'foreign',
    MOCK_VOLUME_OWNER: 'someone-else',
  });

  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /資料磁碟區.*不屬於此管理工具/);
  assert.ok(!calls.some(call => /^(rm |volume rm)/.test(call)), calls.join('\n'));
});


test('up refuses to mount a same-named volume without the fork ownership labels', t => {
  const { result, calls } = runWithMockDocker(t, ['up', 'alice', '--no-open'], {
    MOCK_CONTAINER: 'absent',
    MOCK_VOLUME: 'foreign',
    MOCK_VOLUME_OWNER: 'someone-else',
  });

  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /資料磁碟區.*不屬於此管理工具/);
  assert.ok(!calls.some(call => /^(run |volume create|volume rm|rm )/.test(call)), calls.join('\n'));
  assert.ok(!calls.some(call => call.startsWith('image inspect') || call.startsWith('build ')), calls.join('\n'));
});

test('HTTPS import hides child diagnostics containing the SAS query and keeps its failure status', t => {
  const sasQuery = '?sv=2026-01-01&sig=mock-secret';
  const url = `https://user:private-token@storage.example.org/alice.json${sasQuery}`;
  const { result } = runWithMockDocker(t, ['import', 'alice', url], {
    MOCK_CONTAINER: 'owned',
    MOCK_VOLUME: 'owned',
  });

  assert.equal(result.status, 1);
  assert.match(result.stderr, /結束碼 23/);
  assert.doesNotMatch(`${result.stdout}\n${result.stderr}`, /user:|private-token|mock-secret|sig=/);
});
test('HTTPS import preserves success while keeping the SAS query out of output', t => {
  const sasQuery = '?sv=2026-01-01&sig=mock-secret';
  const url = `https://user:private-token@storage.example.org/alice.json${sasQuery}`;
  const { result } = runWithMockDocker(t, ['import', 'alice', url], {
    MOCK_CONTAINER: 'owned',
    MOCK_VOLUME: 'owned',
    MOCK_EXEC_STATUS: '0',
  });

  assert.equal(result.status, 0);
  assert.match(result.stdout, /匯入 HTTPS SAS 網址/);
  assert.doesNotMatch(`${result.stdout}\n${result.stderr}`, /user:|private-token|mock-secret|sig=/);
});

test('HTTPS export hides child diagnostics containing the SAS query and keeps its failure status', t => {
  const sasQuery = '?sv=2026-01-01&sig=mock-secret';
  const url = `https://user:private-token@storage.example.org/alice.json${sasQuery}`;
  const { result } = runWithMockDocker(t, ['export', 'alice', '--out', url], {
    MOCK_CONTAINER: 'owned',
    MOCK_VOLUME: 'owned',
  });

  assert.equal(result.status, 1);
  assert.match(result.stderr, /結束碼 23/);
  assert.doesNotMatch(`${result.stdout}\n${result.stderr}`, /user:|private-token|mock-secret|sig=/);
});
test('HTTPS export preserves success without printing the SAS URL', t => {
  const url = 'https://user:private-token@storage.example.org/alice.json?sv=2026-01-01&sig=mock-secret';
  const { result } = runWithMockDocker(t, ['export', 'alice', '--out', url], {
    MOCK_CONTAINER: 'owned',
    MOCK_VOLUME: 'owned',
    MOCK_EXEC_STATUS: '0',
  });

  assert.equal(result.status, 0);
  assert.match(result.stdout, /已匯出至指定的 HTTPS Blob 路徑/);
  assert.doesNotMatch(`${result.stdout}\n${result.stderr}`, /user:|private-token|mock-secret|sig=/);
});

test('HTTP import and export errors do not echo URL credentials or query strings', t => {
  const url = 'http://user:private-token@storage.example.org/alice.json?sig=mock-secret';
  const settings = { MOCK_CONTAINER: 'owned', MOCK_VOLUME: 'owned' };
  const importResult = runWithMockDocker(t, ['import', 'alice', url], settings).result;
  const exportResult = runWithMockDocker(t, ['export', 'alice', '--out', url], settings).result;

  for (const result of [importResult, exportResult]) {
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /必須使用 HTTPS/);
    assert.doesNotMatch(`${result.stdout}\n${result.stderr}`, /user:|private-token|mock-secret|sig=/);
  }
});



test('Docker image disables updates and uses a non-root user', () => {
  const dockerfile = readFileSync(dockerfilePath, 'utf8');
  assert.match(dockerfile, /ENTRYPOINT .*--no-auto-update/);
  assert.match(dockerfile, /^USER app$/m);
  assert.doesNotMatch(dockerfile, /^COPY.*VERSION/m);
});

test('Docker build context contains every local COPY source', () => {
  const dockerfile = readFileSync(dockerfilePath, 'utf8');
  for (const [, sources] of dockerfile.matchAll(/^COPY (?!--from=)(.+) \S+$/gm)) {
    for (const source of sources.split(/\s+/)) {
      assert.ok(existsSync(new URL(`../${source}`, import.meta.url)), `missing COPY source: ${source}`);
    }
  }
  assert.doesNotMatch(dockerfile, /APP_BUILD_DATE|build\.rs/);
  assert.doesNotMatch(readFileSync(scriptPath, 'utf8'), /APP_BUILD_DATE/);
});
