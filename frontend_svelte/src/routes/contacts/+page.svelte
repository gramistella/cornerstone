<script lang="ts">
	import { goto } from '$app/navigation';
	import { delApi } from '$lib/api';

	export let data;

	async function deleteContact(id: number) {
		if (confirm('Are you sure you want to delete this contact?')) {
			try {
				await delApi(`contacts/${id}`);
				// A simple way to refresh the data is to re-run the load function
				// For a better UX, you could remove the item from the array directly.
				data.contacts = data.contacts.filter(c => c.id !== id);
			} catch (error) {
				console.error('Failed to delete contact:', error);
				alert('Could not delete contact.');
			}
		}
	}
</script>

<div class="container mx-auto p-4">
	<div class="flex justify-between items-center mb-4">
		<h1 class="text-3xl font-bold">Your Contacts</h1>
		<button on:click={() => goto('/contacts/new')} class="bg-green-500 hover:bg-green-700 text-white font-bold py-2 px-4 rounded">
			Add New Contact
		</button>
	</div>

	{#if data.error}
		<p class="text-red-500">{data.error}</p>
	{:else if data.contacts.length === 0}
		<p>No contacts found. Add one!</p>
	{:else}
		<div class="overflow-x-auto relative shadow-md sm:rounded-lg">
			<table class="w-full text-sm text-left text-gray-500">
				<thead class="text-xs text-gray-700 uppercase bg-gray-50">
					<tr>
						<th scope="col" class="py-3 px-6">Name</th>
						<th scope="col" class="py-3 px-6">Email</th>
						<th scope="col" class="py-3 px-6">Age</th>
						<th scope="col" class="py-3 px-6">Subscribed</th>
						<th scope="col" class="py-3 px-6">Type</th>
						<th scope="col" class="py-3 px-6">Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each data.contacts as contact (contact.id)}
						<tr class="bg-white border-b hover:bg-gray-50">
							<td class="py-4 px-6">{contact.name}</td>
							<td class="py-4 px-6">{contact.email}</td>
							<td class="py-4 px-6">{contact.age}</td>
							<td class="py-4 px-6">{contact.subscribed ? 'Yes' : 'No'}</td>
							<td class="py-4 px-6">{contact.contactType}</td>
							<td class="py-4 px-6">
								<a href="/contacts/{contact.id}" class="font-medium text-blue-600 hover:underline mr-3">Edit</a>
								<button on:click={() => deleteContact(contact.id)} class="font-medium text-red-600 hover:underline">Delete</button>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</div>