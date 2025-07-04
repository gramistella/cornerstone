import { writable } from 'svelte/store';
import { browser } from '$app/environment';
import { jwtDecode } from 'jwt-decode';

const initialTokens = browser ? localStorage.getItem('tokens') : null;

export const tokens = writable<{ accessToken: string; refreshToken: string } | null>(
	initialTokens ? JSON.parse(initialTokens) : null
);

tokens.subscribe((value) => {
	if (browser) {
		if (value) {
			localStorage.setItem('tokens', JSON.stringify(value));
		} else {
			localStorage.removeItem('tokens');
		}
	}
});

// Helper function to get user from access token
export function getUserFromToken(tokenValue: string | null): { sub: string } | null {
	if (!tokenValue) return null;
	try {
		return jwtDecode<{ sub: string; exp: number }>(tokenValue);
	} catch (e) {
		console.error('Invalid token', e);
		return null;
	}
}