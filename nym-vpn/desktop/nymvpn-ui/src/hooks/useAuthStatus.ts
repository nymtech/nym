import { invoke } from '@tauri-apps/api'
import React, { useEffect, useState } from 'react';
import { isOffline } from '../lib/util';
import { UiError } from '../lib/types';

function delay(ms: number) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

function useAuthStatus() {
    const [checkingAuthStatus, setCheckingAuthStatus] = useState(true)
    const [daemonOffline, setDaemonOffline] = useState(false)
    const [signedIn, setSignedIn] = useState(false)

    // periodically check for auth status as well as offline status
    useEffect(() => {
        const id = setInterval(() => {
            async function check() {
                try {
                    const isSignedIn = await invoke('is_signed_in') as boolean
                    setSignedIn(isSignedIn);
                    setDaemonOffline(false);
                } catch (e) {
                    if (isOffline(e as UiError)) {
                        setDaemonOffline(true)
                    }
                }
            }
            check();
        }, 30000);

        return () => clearInterval(id);
    }, [])

    // initial check
    useEffect(() => {
        const fetchSignIn = async () => {
            let retries = 10;

            while (retries > 0) {
                try {
                    const isSignedIn = await invoke('is_signed_in') as boolean
                    setSignedIn(isSignedIn)
                    break;
                } catch (e) {
                    retries -= 1
                    if (retries == 0) {
                        setDaemonOffline(true)
                        setSignedIn(false)
                    }
                }
                await delay(1000);
            }
            setCheckingAuthStatus(false)
        }

        fetchSignIn()
    }, [])


    return [checkingAuthStatus, daemonOffline, signedIn]
}

export default useAuthStatus
