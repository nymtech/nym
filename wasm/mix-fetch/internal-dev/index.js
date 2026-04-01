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
        updateStatus('mixfetch-status', 'Starting...', 'orange');

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
        updateStatus('stress-test-status', 'Running...', 'orange');
        client.setGoTimeout(goTimeoutMs);

        const requests = generateStressRequests(count, mode, goTimeoutMs);
        stressTest = {
            count,
            mode,
            startTime: performance.now(),
            results: [],
            completionOrder: [],
            profiles: {},
        };
        for (const req of requests) {
            stressTest.profiles[req.id] = { label: req.label, bytes: req.bytes };
        }

        initStressTracker(requests);

        console.log(`%c=== STRESS TEST: ${count} requests, ${mode} mode, timeout=${goTimeoutMs}ms ===`, 'font-weight: bold');

        if (mode === 'mixed' || mode === 'drip') {
            const breakdown = {};
            for (const req of requests) breakdown[req.label] = (breakdown[req.label] || 0) + 1;
            console.log('Profiles:', breakdown);
        }

        client.doStressTest(requests);
    };
}

// ─── UI helpers ─────────────────────────────────────────────────────────────

function updateStatus(elementId, text, color) {
    const el = document.getElementById(elementId);
    el.textContent = text;
    el.style.color = color;
}

function onMixFetchReady() {
    updateStatus('mixfetch-status', 'Ready', 'green');
    document.getElementById('fetch-controls').disabled = false;
    console.log('%cMixFetch ready!', 'color: green; font-weight: bold');
}

function onMixFetchError(error) {
    updateStatus('mixfetch-status', 'Error: ' + error, 'red');
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

// Per-profile live tracker state
let trackerData = {};

function initStressTracker(requests) {
    const tracker = document.getElementById('stress-tracker');
    tracker.style.display = 'block';
    trackerData = {};

    // Count how many of each profile were sent
    for (const req of requests) {
        if (!trackerData[req.label]) {
            trackerData[req.label] = { sent: 0, ok: 0, fail: 0, times: [] };
        }
        trackerData[req.label].sent++;
    }

    renderTracker();
}

function renderTracker() {
    const tracker = document.getElementById('stress-tracker');
    const rows = Object.entries(trackerData).map(([label, d]) => {
        const pending = d.sent - d.ok - d.fail;
        const avg = d.times.length > 0
            ? (d.times.reduce((a, b) => a + b, 0) / d.times.length).toFixed(1) + 's'
            : '-';
        const max = d.times.length > 0 ? Math.max(...d.times).toFixed(1) + 's' : '-';
        const min = d.times.length > 0 ? Math.min(...d.times).toFixed(1) + 's' : '-';

        let status = '';
        if (d.ok > 0) status += `<span style="color: green">${d.ok} ok</span>`;
        if (d.fail > 0) status += `${status ? ' ' : ''}<span style="color: red">${d.fail} fail</span>`;
        if (pending > 0) status += `${status ? ' ' : ''}<span style="color: gray">${pending} pending</span>`;

        let timing = '';
        if (d.times.length > 0) {
            timing = `avg ${avg} / min ${min} / max ${max}`;
        }

        return `<tr>
            <td style="padding: 2px 8px; font-weight: bold">${label}</td>
            <td style="padding: 2px 8px">${d.sent}</td>
            <td style="padding: 2px 8px">${status}</td>
            <td style="padding: 2px 8px; color: #666">${timing}</td>
        </tr>`;
    });

    tracker.innerHTML = `<table style="border-collapse: collapse">
        <tr style="border-bottom: 1px solid #ccc">
            <th style="padding: 2px 8px; text-align: left">profile</th>
            <th style="padding: 2px 8px; text-align: left">sent</th>
            <th style="padding: 2px 8px; text-align: left">status</th>
            <th style="padding: 2px 8px; text-align: left">timing</th>
        </tr>
        ${rows.join('')}
    </table>`;
}

function onStressTestFetchResult(result) {
    if (!stressTest) return;

    const profile = stressTest.profiles[result.id];
    result.label = profile ? profile.label : '?';

    stressTest.results.push(result);
    stressTest.completionOrder.push(result.id);

    // Update tracker
    const td = trackerData[result.label];
    if (td) {
        if (result.ok) {
            td.ok++;
            td.times.push(parseFloat(result.elapsed));
        } else {
            td.fail++;
        }
        renderTracker();
    }

    const progress = `${stressTest.results.length}/${stressTest.count}`;
    const tag = `#${result.id} ${result.label}`;

    if (result.ok) {
        console.log(`%c[${tag}] ${result.status} OK ${result.elapsed}s ${result.textLength}B  (${progress})`, 'color: green');
    } else {
        console.error(`[${tag}] FAIL ${result.elapsed}s ${result.error}  (${progress})`);
    }

    updateStatus('stress-test-status', progress, 'orange');

    // Summary on completion
    if (stressTest.results.length === stressTest.count) {
        const totalElapsed = ((performance.now() - stressTest.startTime) / 1000).toFixed(2);
        const succeeded = stressTest.results.filter((r) => r.ok).length;
        const failed = stressTest.results.filter((r) => !r.ok).length;

        console.log(`%c=== COMPLETE: ${totalElapsed}s | OK ${succeeded}/${stressTest.count} | Failed ${failed}/${stressTest.count} ===`, 'font-weight: bold');
        console.log('Completion order:', stressTest.completionOrder);

        // Per-profile breakdown in console
        const profileLabels = Object.keys(trackerData);
        const profileList = profileLabels.map((label) => ({ label }));
        if (stressTest.mode === 'mixed' || stressTest.mode === 'drip') {
            const table = [];
            for (const profile of profileList) {
                const matching = stressTest.results.filter((r) => r.label === profile.label);
                if (matching.length === 0) continue;
                const ok = matching.filter((r) => r.ok).length;
                const times = matching.filter((r) => r.ok).map((r) => parseFloat(r.elapsed));
                const avg = times.length > 0 ? (times.reduce((a, b) => a + b, 0) / times.length).toFixed(2) : '-';
                const max = times.length > 0 ? Math.max(...times).toFixed(2) : '-';
                table.push({ profile: profile.label, ok: `${ok}/${matching.length}`, avg: `${avg}s`, max: `${max}s` });
            }
            console.table(table);
        }

        updateStatus('stress-test-status',
            `Done: ${succeeded}/${stressTest.count} OK, ${failed} failed (${totalElapsed}s)`,
            failed > 0 ? 'red' : 'green'
        );
        document.querySelector('#stress-test-button').disabled = false;
        stressTest = null;
    }
}

main();
