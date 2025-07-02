import { redirect } from '@sveltejs/kit';
import { get } from 'svelte/store';
import { token } from '$lib/auth';
import { browser } from '$app/environment';

export const load = () => {
    // We only run this on the client side, after the auth store has been initialized.
    if (browser) {
        const currentToken = get(token);
        if (!currentToken) {
            throw redirect(307, '/login');
        }
    }
};