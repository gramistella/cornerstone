<script lang="ts">
	import { goto } from '$app/navigation';
	import { postApi } from '$lib/api';
	import type { ContactDto } from '$lib/types';

	let contact: Partial<ContactDto> = {
		name: '',
		email: '',
		age: undefined,
		subscribed: false,
		contactType: 'Personal'
	};
	let errorMessage = '';

	async function handleSubmit() {
		errorMessage = '';
		try {
			await postApi('contacts', contact);
			goto('/contacts', { invalidateAll: true });
		} catch (error) {
			errorMessage = (error as Error).message || 'Failed to create contact.';
			console.error('Create error:', error);
		}
	}
</script>

<div class="max-w-md mx-auto mt-10 p-6 bg-white rounded-lg shadow-md">
	<h2 class="text-2xl font-bold mb-6 text-center">Add New Contact</h2>
	<form on:submit|preventDefault={handleSubmit}>
		{#if errorMessage}
			<div class="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded relative mb-4" role="alert">
				<span class="block sm:inline">{errorMessage}</span>
			</div>
		{/if}
		<div class="mb-4">
			<label for="name" class="block text-gray-700">Name</label>
			<input type="text" id="name" bind:value={contact.name} class="w-full px-3 py-2 border rounded" required />
		</div>
		<div class="mb-4">
			<label for="email" class="block text-gray-700">Email</label>
			<input type="email" id="email" bind:value={contact.email} class="w-full px-3 py-2 border rounded" required />
		</div>
		<div class="mb-4">
			<label for="age" class="block text-gray-700">Age</label>
			<input type="number" id="age" bind:value={contact.age} class="w-full px-3 py-2 border rounded" required />
		</div>
        <div class="mb-4">
			<label for="contactType" class="block text-gray-700">Contact Type</label>
			<select id="contactType" bind:value={contact.contactType} class="w-full px-3 py-2 border rounded">
				<option value="Personal">Personal</option>
				<option value="Work">Work</option>
				<option value="Other">Other</option>
			</select>
		</div>
		<div class="mb-6 flex items-center">
			<input type="checkbox" id="subscribed" bind:checked={contact.subscribed} class="mr-2" />
			<label for="subscribed" class="text-gray-700">Subscribed to newsletter</label>
		</div>
		<button type="submit" class="w-full bg-green-500 hover:bg-green-700 text-white font-bold py-2 px-4 rounded">
			Create Contact
		</button>
	</form>
</div>