import hasha from "hasha";
import fetch from "node-fetch";
import { Octokit } from "@octokit/rest";
import fs from "fs";
import path from "path";
import { execSync } from "child_process";

function getBinInfo(path) {
    // let's be super naive about it. add a+x bits on the file and try to run the command
    try {
        let mode = fs.statSync(path).mode
        fs.chmodSync(path, mode | 0o111)

        const raw = execSync(`${path} build-info --output=json`, { stdio: 'pipe', encoding: "utf8" });
        const parsed = JSON.parse(raw)
        return parsed
    } catch (_) {
        return undefined
    }
}

async function run(assets, algorithm, filename, cache) {
    if (!cache) {
        console.warn("cache is set to 'false', but we we no longer support it")
    }

    try {
        fs.mkdirSync('.tmp');
    } catch(e) {
        // ignore
    }

    const hashes = {};
    let numAwaiting = 0;
    for (const asset of assets) {
        if (filename === "" || asset.name !== filename) { // don't hash the hash file (if the file has the same name)
            numAwaiting++;

            let buffer = null;
            let sig = null;

            // cache in `${WORKING_DIR}/.tmp/`
            const cacheFilename = path.resolve(`.tmp/${asset.name}`);
            if(!fs.existsSync(cacheFilename)) {
                console.log(`Downloading ${asset.browser_download_url}... to ${cacheFilename}`);
                buffer = Buffer.from(await fetch(asset.browser_download_url).then(res => res.arrayBuffer()));
                fs.writeFileSync(cacheFilename, buffer);
            } else {
                console.log(`Loading from ${cacheFilename}`);
                buffer = Buffer.from(fs.readFileSync(cacheFilename));

                // console.log('Reading signature from content');
                // if(asset.name.endsWith('.sig')) {
                //     sig = fs.readFileSync(cacheFilename).toString();
                // }
            }

            const binInfo = getBinInfo(cacheFilename)

            if(!hashes[asset.name]) {
                hashes[asset.name] = {};
            }

            if(asset.name.endsWith('.sig')) {
                sig = buffer.toString();
            }

            hashes[asset.name][algorithm] = hasha(new Uint8Array(buffer), {algorithm: algorithm});

            let platform;
            let kind;
            if(asset.name.endsWith('.sig')) {
                kind = 'signature';
            }
            if(asset.name.endsWith('.app.tar.gz')) {
                platform = 'MacOS';
                kind = 'auto-updater';
            }
            if(asset.name.endsWith('.app.tar.gz.sig')) {
                platform = 'MacOS';
                kind = 'auto-updater-signature';
            }
            if(asset.name.endsWith('.dmg')) {
                platform = 'MacOS';
                kind = 'installer';
            }
            if(asset.name.endsWith('.msi.zip')) {
                platform = 'Windows';
                kind = 'auto-updater';
            }
            if(asset.name.endsWith('.msi.zip.sig')) {
                platform = 'Windows';
                kind = 'auto-updater-signature';
            }
            if(asset.name.endsWith('.msi')) {
                platform = 'Windows';
                kind = 'installer';
            }
            if(asset.name.endsWith('.AppImage.tar.gz')) {
                platform = 'Linux';
                kind = 'auto-updater';
            }
            if(asset.name.endsWith('.AppImage.tar.gz.sig')) {
                platform = 'Linux';
                kind = 'auto-updater-signature';
            }
            if(asset.name.endsWith('.AppImage')) {
                platform = 'Linux';
                kind = 'installer';
            }

            hashes[asset.name].downloadUrl = asset.browser_download_url;

            if(platform) {
                hashes[asset.name].platform = platform;
            }
            if(kind) {
                hashes[asset.name].kind = kind;
            }
            if(binInfo) {
                hashes[asset.name].details = binInfo;
            }

            // process Tauri signature files
            if(asset.name.endsWith('.sig')) {
                const otherFilename = asset.name.replace('.sig', '');
                if(!hashes[otherFilename]) {
                    hashes[otherFilename] = {};
                }
                hashes[otherFilename].signature = sig;
            }
        }
    }
    return hashes;
}

export async function createHashes({ assets, algorithm, filename, cache }) {
    const output = await run(assets, algorithm, filename, cache);
    if(filename?.length) {
        fs.writeFileSync(filename, JSON.stringify(output, null, 2));
    }
    return output;
}

export async function createHashesFromReleaseTagOrNameOrId({ releaseTagOrNameOrId, algorithm = 'sha256', filename = 'hashes.json', cache = false, upload = true, owner = 'nymtech', repo = 'nym' }) {
    console.log("🚀🚀🚀 Getting releases");

    let auth;
    let authStrategy;
    if(process.env.GITHUB_TOKEN) {
        console.log('Using GITHUB_TOKEN for auth');
        // authStrategy = createActionAuth();
        // auth = await authStrategy();
    }

    const octokit = new Octokit({
        auth: process.env.GITHUB_TOKEN,
        request: { fetch }
    });

    let releases;
    if(cache) {
        const cacheFilename = path.resolve(`.tmp/releases.json`);
        if(!fs.existsSync(cacheFilename)) {
            releases = await octokit.paginate(
                octokit.rest.repos.listReleases,
                {
                    owner,
                    repo,
                    per_page: 100,
                },
                (response) => response.data
            );
            fs.writeFileSync(cacheFilename, JSON.stringify(releases, null, 2));
        } else {
            console.log('Loading releases from cache...');
            releases = JSON.parse(fs.readFileSync(cacheFilename));
        }
    } else {
        releases = await octokit.paginate(
            octokit.rest.repos.listReleases,
            {
                owner,
                repo,
                per_page: 100,
            },
            (response) => response.data
        )
    }

    // process all releases by default
    let releasesToProcess = releases;

    // process a single release
    if(releaseTagOrNameOrId) {
        releasesToProcess = releases.filter(r => {
            if (r.tag_name === releaseTagOrNameOrId) {
                return true;
            }
            if (`${r.id}` === `${releaseTagOrNameOrId}`) {
                return true;
            }
            if (r.name === releaseTagOrNameOrId) {
                return true;
            }

            return false;
        });
    }

    releasesToProcess.forEach(release => {
        const {tag_name, name} = release;
        const matches = tag_name.match(/(\S+)-v([0-9]+\.[0-9]+(\.\S+)?)/);

        if(!matches || matches.length < 2) {
            console.warn('Could not match version structure in tag name = ', tag_name);
            return;
        }

        const tagComponents = matches.slice(1);
        const componentName = tagComponents[0];
        const componentVersion = 'v' + tagComponents[1];

        if(!tagComponents[1] || !name) {
            return;
        }

        release.componentName = componentName;
        release.componentVersion = componentVersion;
    })

    releasesToProcess = releasesToProcess.filter(release =>
        !!release.name && !!release.componentVersion
    );

    console.log('Releases to process:');
    console.table(releasesToProcess.map(r => {
        const { id, name, tag_name, componentName, componentVersion, assets } = r;
        return { id, name, tag_name, componentName, componentVersion, assetCount: assets.length };
    }));

   for(const release of releasesToProcess) {
        const {id, name, tag_name, html_url, componentName, componentVersion} = release;

        const hashes = await createHashes({ assets: release.assets, algorithm, filename, cache });

        const output = {
            id, name, tag_name, html_url,
            componentName,
            componentVersion,
            assets: hashes,
        };

        console.log(output)

        if(upload) {
            console.log(`🚚 Uploading ${filename} to release name="${release.name}" id=${release.id} (${release.upload_url})...`);

            const exists = (await octokit.repos.listReleaseAssets({ owner, repo, release_id: release.id })).data.find(a => a.name === filename)
            if (exists) {
                console.log(`Deleting existing asset ${filename}...`);
                await octokit.repos.deleteReleaseAsset({ owner, repo, asset_id: exists.id })
                console.log('Deleted existing asset');
            }

            try {
                const data = JSON.stringify(output, null, 2);
                await octokit.rest.repos.uploadReleaseAsset({
                    owner,
                    repo,
                    release_id: release.id,
                    headers: {
                        'X-GitHub-Api-Version': '2022-11-28'
                    },
                    name: filename,
                    data,
                });
                console.log('✅ Upload to release is complete.');
            } catch(e) {
                console.log('❌ failed to upload:', e.message, e.status, e.response.data);
                console.log(e);
                process.exit(-1);
            }
        }
   }
}

