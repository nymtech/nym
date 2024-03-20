import {createHashesFromReleaseTagOrNameOrId} from './create-hashes.mjs';

await createHashesFromReleaseTagOrNameOrId({releaseTagOrNameOrId: 119065724, cache: true, upload: false});
await createHashesFromReleaseTagOrNameOrId({releaseTagOrNameOrId: '119065724', cache: true, upload: false});
await createHashesFromReleaseTagOrNameOrId({releaseTagOrNameOrId: 'nym-connect-v1.1.19-snickers', cache: true, upload: false});
await createHashesFromReleaseTagOrNameOrId({releaseTagOrNameOrId: 'Nym Connect v1.1.19-snickers', cache: true, upload: false});
