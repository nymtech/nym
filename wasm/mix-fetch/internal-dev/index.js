// Copyright 2020-2023 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// ─── Worker client ──────────────────────────────────────────────────────────

class WebWorkerClient {
    worker = null;

    constructor() {
        this.worker = new Worker('./worker.js');

        this.worker.onmessage = (ev) => {
            if (!ev.data || !ev.data.kind) return;
            switch (ev.data.kind) {
                case 'DisplayString':
                    appendFetchLog(ev.data.args.rawString);
                    console.log('[mixfetch response]', ev.data.args.rawString);
                    break;
                case 'Log': {
                    const { message, level } = ev.data.args;
                    const fn = level === 'error' ? console.error
                             : level === 'warn' ? console.warn
                             : console.log;
                    fn(`[worker/${level}]`, message);
                    break;
                }
                case 'MixFetchReady':
                    onMixFetchReady();
                    break;
                case 'MixFetchError':
                    onMixFetchError(ev.data.args.error);
                    break;
                case 'StressTestFetchResult':
                    onStressTestFetchResult(ev.data.args);
                    break;
            }
        };
    }

    startMixFetch = (preferredGateway, setupOpts) => {
        this.worker.postMessage({ kind: 'StartMixFetch', args: { preferredGateway, setupOpts } });
    };

    doFetch = (target) => {
        this.worker.postMessage({ kind: 'FetchPayload', args: { target } });
    };

    setGoTimeout = (timeoutMs) => {
        this.worker.postMessage({ kind: 'SetGoTimeout', args: { timeoutMs } });
    };

    doStressTest = (requests) => {
        for (const req of requests) {
            this.worker.postMessage({
                kind: 'StressTestFetch',
                args: { id: req.id, url: req.url, label: req.label },
            });
        }
    };
}

// ─── Startup ────────────────────────────────────────────────────────────────

let client = null;
const DEFAULT_GATEWAY = 'q2A2cbooyC16YJzvdYaSMH9X3cSiieZNtfBr8cE8Fi1';

async function main() {
    client = new WebWorkerClient();

    // Randomise client ID on each load to avoid storage/state collisions
    document.getElementById('opt-client-id').value =
        'client-' + Math.random().toString(36).slice(2, 8);

    document.querySelector('#start-mixfetch').onclick = () => {
        const gatewayMode = document.querySelector('input[name="gateway-mode"]:checked').value;
        const preferredGateway = gatewayMode === 'default' ? DEFAULT_GATEWAY : undefined;

        const setupOpts = {
            forceTls: document.getElementById('opt-force-tls').checked,
            clientId: document.getElementById('opt-client-id').value,
            disablePoisson: document.getElementById('opt-disable-poisson').checked,
            disableCover: document.getElementById('opt-disable-cover').checked,
            requestTimeoutMs: parseInt(document.getElementById('opt-request-timeout').value, 10),
        };

        document.querySelector('#start-mixfetch').disabled = true;
        document.querySelectorAll('input[name="gateway-mode"]').forEach((r) => (r.disabled = true));
        updateStatus('mixfetch-status', 'Starting...');

        // Sync the stress-test Go timeout to match the configured request timeout
        document.getElementById('stress-go-timeout').value = setupOpts.requestTimeoutMs;

        console.log(`Starting MixFetch (${gatewayMode} gateway${preferredGateway ? `: ${preferredGateway}` : ''})...`);
        console.log('Setup options:', setupOpts);
        client.startMixFetch(preferredGateway, setupOpts);
    };

    document.querySelector('#fetch-button-1').onclick = () => doFetch(1);
    document.querySelector('#fetch-button-2').onclick = () => doFetch(2);

    const stressModeSelect = document.getElementById('stress-test-mode');
    stressModeSelect.onchange = function () {
        document.getElementById('stress-uniform-opts').style.display = this.value === 'uniform' ? 'block' : 'none';
        document.getElementById('stress-mixed-opts').style.display = this.value === 'mixed' ? 'block' : 'none';
        document.getElementById('stress-drip-opts').style.display = this.value === 'drip' ? 'block' : 'none';
    };

    document.querySelector('#stress-test-button').onclick = () => {
        const count = parseInt(document.getElementById('stress-test-count').value, 10);
        const mode = document.getElementById('stress-test-mode').value;
        const goTimeoutMs = parseInt(document.getElementById('stress-go-timeout').value, 10);

        document.querySelector('#stress-test-button').disabled = true;
        updateStatus('stress-test-status', 'Running...');
        client.setGoTimeout(goTimeoutMs);

        const requests = generateStressRequests(count, mode, goTimeoutMs);
        stressTest = {
            count,
            startTime: performance.now(),
            results: [],
        };

        console.log(`=== STRESS TEST: ${count} requests, ${mode} mode, timeout=${goTimeoutMs}ms ===`);

        if (mode === 'mixed' || mode === 'drip') {
            const breakdown = {};
            for (const req of requests) breakdown[req.label] = (breakdown[req.label] || 0) + 1;
            console.log('Profiles:', breakdown);
        }

        client.doStressTest(requests);
    };
}

// ─── UI helpers ─────────────────────────────────────────────────────────────

function updateStatus(elementId, text) {
    document.getElementById(elementId).textContent = text;
}

function onMixFetchReady() {
    updateStatus('mixfetch-status', 'Ready');
    document.getElementById('fetch-controls').disabled = false;
    console.log('MixFetch ready!');
}

function onMixFetchError(error) {
    updateStatus('mixfetch-status', 'Error: ' + error);
    document.querySelector('#start-mixfetch').disabled = false;
    document.querySelectorAll('input[name="gateway-mode"]').forEach((r) => (r.disabled = false));
    console.error('MixFetch error:', error);
}

// ─── Quick fetch ────────────────────────────────────────────────────────────

function appendFetchLog(text) {
    const log = document.getElementById('fetch-log');
    log.style.display = 'block';
    const ts = new Date().toISOString().substr(11, 12);
    log.textContent += `${ts}  ${text}\n`;
    log.scrollTop = log.scrollHeight;
}

async function doFetch(id) {
    const url = document.getElementById(`fetch_payload_${id}`).value;
    appendFetchLog(`GET ${url}`);
    console.log(`GET ${url}`);
    await client.doFetch(url);
}

// ─── Stress test ────────────────────────────────────────────────────────────

const STRESS_PROFILES = [
    { label: 'tiny', bytes: 128 },
    { label: 'small', bytes: 1024 },
    { label: 'medium', bytes: 10240 },
    { label: 'large', bytes: 102400 },
    { label: 'xlarge', bytes: 1048576 },
];

function buildDripProfiles(timeoutSec) {
    return [
        { label: 'safe',       dripDuration: Math.round(timeoutSec * 0.50), dripDelay: 0, dripBytes: 100 },
        { label: 'boundary',   dripDuration: Math.round(timeoutSec * 0.92), dripDelay: 0, dripBytes: 100 },
        { label: 'over',       dripDuration: Math.round(timeoutSec * 1.08), dripDelay: 0, dripBytes: 100 },
        { label: 'slow-start', dripDuration: Math.round(timeoutSec * 0.83), dripDelay: Math.round(timeoutSec * 0.17), dripBytes: 100 },
    ];
}

function generateStressRequests(count, mode, timeoutMs) {
    const requests = [];
    if (mode === 'uniform') {
        const baseUrl = document.getElementById('stress-test-url').value;
        for (let i = 1; i <= count; i++) {
            requests.push({ id: i, url: `${baseUrl}${i}`, label: 'uniform', bytes: null });
        }
    } else if (mode === 'drip') {
        const dripProfiles = buildDripProfiles(timeoutMs / 1000);
        for (let i = 1; i <= count; i++) {
            const p = dripProfiles[Math.floor(Math.random() * dripProfiles.length)];
            requests.push({
                id: i,
                url: `https://httpbin.org/drip?duration=${p.dripDuration}&numbytes=${p.dripBytes}&delay=${p.dripDelay}&code=200`,
                label: p.label,
                bytes: p.dripBytes,
            });
        }
    } else {
        for (let i = 1; i <= count; i++) {
            const p = STRESS_PROFILES[Math.floor(Math.random() * STRESS_PROFILES.length)];
            requests.push({
                id: i,
                url: `https://httpbin.org/bytes/${p.bytes}`,
                label: p.label,
                bytes: p.bytes,
            });
        }
    }
    return requests;
}

let stressTest = null;

function onStressTestFetchResult(result) {
    if (!stressTest) return;

    stressTest.results.push(result);

    const progress = `${stressTest.results.length}/${stressTest.count}`;
    const tag = `#${result.id} ${result.label}`;

    if (result.ok) {
        console.log(`[${tag}] ${result.status} OK ${result.elapsed}s ${result.textLength}B  (${progress})`);
    } else {
        console.error(`[${tag}] FAIL ${result.elapsed}s ${result.error}  (${progress})`);
    }

    updateStatus('stress-test-status', progress);

    if (stressTest.results.length === stressTest.count) {
        const totalElapsed = ((performance.now() - stressTest.startTime) / 1000).toFixed(2);
        const succeeded = stressTest.results.filter((r) => r.ok).length;
        const failed = stressTest.results.filter((r) => !r.ok).length;

        console.log(`=== COMPLETE: ${totalElapsed}s | OK ${succeeded}/${stressTest.count} | Failed ${failed}/${stressTest.count} ===`);

        if (failed > 0) {
            const failures = stressTest.results.filter((r) => !r.ok);
            for (const f of failures) {
                console.log(`  FAIL #${f.id} ${f.label} (${f.elapsed}s): ${f.error}`);
            }
        }

        updateStatus('stress-test-status',
            `Done: ${succeeded}/${stressTest.count} OK, ${failed} failed (${totalElapsed}s)`
        );
        document.querySelector('#stress-test-button').disabled = false;
        stressTest = null;
    }
}

main();
