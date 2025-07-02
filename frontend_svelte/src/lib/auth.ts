import { writable } from 'svelte/store';
import { browser } from '$app/environment';
import { jwtDecode } from 'jwt-decode';

// Initialize token from localStorage if in browser
const initialToken = browser ? window.localStorage.getItem('token') : null;
export const token = writable<string | null>(initialToken);

// Subscribe to token changes and update localStorage
token.subscribe((value) => {
	if (browser) {
		if (value) {
			window.localStorage.setItem('token', value);
		} else {
			window.localStorage.removeItem('token');
		}
	}
});

// Helper function to get user data from token
export function getUserFromToken(tokenValue: string | null): { sub: string } | null {
	if (!tokenValue) return null;
	try {
		return jwtDecode<{ sub: string; exp: number }>(tokenValue);
	} catch (e) {
		console.error('Invalid token', e);
		return null;
	}
}