import { getApi } from '$lib/api';

/** @type {import('./$types').PageLoad} */
export async function load({ fetch }) {
	try {
		const contacts = await getApi('contacts', { fetch });
		return {
			contacts
		};
	} catch (error) {
		console.error('Failed to load contacts:', error);
		return {
			contacts: [],
			error: 'Could not load contacts.'
		};
	}
}
