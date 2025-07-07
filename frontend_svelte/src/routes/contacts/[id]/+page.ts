import { getApi } from '$lib/api';
import { error } from '@sveltejs/kit';

/** @type {import('./$types').PageLoad} */
export async function load({ params, fetch }) {
    try {
        const contact = await getApi(`contacts/${params.id}`);
        if (contact) {
            return {
                contact
            };
        }
    } catch (e) {
        throw error(404, 'Not found');
    }
}