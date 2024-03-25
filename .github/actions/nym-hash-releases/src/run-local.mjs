import {createHashesFromReleaseTagOrNameOrId} from './create-hashes.mjs';

const cache = true;

await createHashesFromReleaseTagOrNameOrId({releaseTagOrNameOrId: 'nym-binaries-v2024.1-marabou', cache, upload: false});
await createHashesFromReleaseTagOrNameOrId({releaseTagOrNameOrId: 'nym-vpn-desktop-v0.0.8', cache, upload: false, repo: 'nym-vpn-client'});

// await createHashesFromReleaseTagOrNameOrId({releaseTagOrNameOrId: 119065724, cache: true, upload: false});
// await createHashesFromReleaseTagOrNameOrId({releaseTagOrNameOrId: '119065724', cache: true, upload: false});
// await createHashesFromReleaseTagOrNameOrId({releaseTagOrNameOrId: 'nym-connect-v1.1.19-snickers', cache: true, upload: false});
// await createHashesFromReleaseTagOrNameOrId({releaseTagOrNameOrId: 'Nym Connect v1.1.19-snickers', cache: true, upload: false});
