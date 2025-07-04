import { get, writable } from 'svelte/store';
import { tokens } from '$lib/auth';
import { goto } from '$app/navigation';
import { browser } from '$app/environment';

const BASE_URL = '/api/v1'; // This will be proxied by Vite in dev

const isRefreshing = writable(false);

async function send({ method, path, data }: { method: string; path: string; data?: any }) {
	let currentTokens = get(tokens);

	// If we are already refreshing, wait for it to complete
	if (get(isRefreshing) && path !== 'refresh') {
		await new Promise((resolve) => {
			const unsubscribe = isRefreshing.subscribe((refreshing) => {
				if (!refreshing) {
					unsubscribe();
					resolve(null);
				}
			});
		});
		currentTokens = get(tokens); // Get the new tokens
	}

	const opts: RequestInit = { method, headers: {} };
	if (data) {
		opts.headers['Content-Type'] = 'application/json';
		opts.body = JSON.stringify(data);
	}

	if (currentTokens?.accessToken) {
		opts.headers['Authorization'] = `Bearer ${currentTokens.accessToken}`;
	}

	let res = await fetch(`${BASE_URL}/${path}`, opts);

	// If token is expired (401), try to refresh it
	if (res.status === 401 && path !== 'refresh' && path !== 'login') {
    if (!currentTokens?.refreshToken) {
        tokens.set(null);
        if (browser) await goto('/login');
        throw new Error('No refresh token available.');
    }

    isRefreshing.set(true);

    try {
        // --- THIS IS THE MODIFIED PART ---
        // Send BOTH the expired access token and the refresh token.
        const refreshRes = await fetch(`${BASE_URL}/refresh`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    refresh_token: currentTokens.refreshToken
                })
            });

        if (!refreshRes.ok) {
            tokens.set(null);
            if (browser) await goto('/login');
            throw new Error('Session expired. Please log in again.');
        }

        const newTokens = await refreshRes.json();
        tokens.set({
            accessToken: newTokens.access_token,
            refreshToken: newTokens.refresh_token, // The backend sends back a new refresh token
        });

        // Retry the original request with the new access token.
        opts.headers['Authorization'] = `Bearer ${newTokens.access_token}`;
        res = await fetch(`${BASE_URL}/${path}`, opts);
    } finally {
        isRefreshing.set(false);
    }
}

	if (!res.ok) {
		const errorText = await res.text();
		throw new Error(errorText || `HTTP error! status: ${res.status}`);
	}

	const contentType = res.headers.get('content-type');
	if (contentType?.includes('application/json')) {
		return res.json();
	}

	return; // No content for 204 responses
}

export function getApi(path: string) {
	return send({ method: 'GET', path });
}

export function delApi(path: string) {
	return send({ method: 'DELETE', path });
}

export function postApi(path: string, data: any) {
	return send({ method: 'POST', path, data });
}

export function putApi(path: string, data: any) {
	return send({ method: 'PUT', path, data });
}