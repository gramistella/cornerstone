import { get } from 'svelte/store';
import { token } from '$lib/auth';

const BASE_URL = '/api/v1'; // This will be proxied by Vite in dev

async function send({ method, path, data }: { method: string; path: string; data?: any }) {
	const opts: RequestInit = { method, headers: {} };

	if (data) {
		opts.headers['Content-Type'] = 'application/json';
		opts.body = JSON.stringify(data);
	}

	const currentToken = get(token);
	if (currentToken) {
		opts.headers['Authorization'] = `Bearer ${currentToken}`;
	}

	const res = await fetch(`${BASE_URL}/${path}`, opts);

	if (!res.ok) {
		const errorText = await res.text();
		throw new Error(errorText || `HTTP error! status: ${res.status}`);
	}

	// Handle responses that might not have a body (e.g., 204 No Content)
	const contentType = res.headers.get('content-type');
	if (contentType && contentType.indexOf('application/json') !== -1) {
		return res.json();
	}

	return; // No content
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