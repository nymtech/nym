// timer actions
// Store current time as `start`
export const now = (eventName = null) => {
    if (eventName) {
        console.log(`Started ${eventName}..`);
    }
    return new Date().getTime();
}

//takes arg of start time 
export const elapsed = (beginning: number, log = false) => {
    const duration = new Date().getTime() - beginning;
    if (log) {
        console.log(`${duration / 1000}s`);
    }
    return duration;
}
