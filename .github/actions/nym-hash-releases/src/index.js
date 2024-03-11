import core from "@actions/core";
import github from "@actions/github";
import { createHashesFromReleaseTagOrNameOrId } from './create-hashes.mjs';

const algorithm = core.getInput('hash-type');
const filename = core.getInput("file-name");
const owner = core.getInput("owner");
const repo = core.getInput("repo");

async function main() {
// use the release id from the payload if it is set
    const releaseTagOrNameOrId = core.getInput("release-tag-or-name-or-id") || github.context.payload.release?.id;

    try {
        await createHashesFromReleaseTagOrNameOrId({releaseTagOrNameOrId, algorithm, filename, owner, repo})
    } catch (error) {
        core.setFailed(error.message);
    }
}

main().catch(error => core.setFailed(error.message));
